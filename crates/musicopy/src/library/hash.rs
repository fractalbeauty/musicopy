use crate::database::{Database, FileHash, FileSize, InsertFileHash, InsertFileSize};
use anyhow::Context;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{
    borrow::Cow,
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};
use tracing::warn;

pub(crate) struct CacheKey<'a> {
    file_size: u64,
    modified_at: u64,
    path: &'a Path,
}

impl<'a> CacheKey<'a> {
    fn read_metadata(path: &'a Path) -> anyhow::Result<Self> {
        let metadata = std::fs::metadata(path).context("failed to get file metadata")?;

        let modified_at = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(Self {
            file_size: metadata.len(),
            modified_at,
            path,
        })
    }

    fn matches_file_hash(&self, file_hash: &FileHash) -> bool {
        self.file_size == file_hash.last_file_size && self.modified_at == file_hash.last_modified_at
    }

    fn matches_file_size(&self, file_size: &FileSize) -> bool {
        self.file_size == file_size.last_file_size && self.modified_at == file_size.last_modified_at
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }
}

#[derive(Debug, Clone)]
pub struct HashCache {
    db: Arc<Mutex<Database>>,
}

impl HashCache {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    /// Gets the cache key for a file by reading its metadata (file size and modified time).
    ///
    /// This can be expensive, especially for network files.
    pub(crate) fn read_cache_key<'a>(&self, path: &'a Path) -> anyhow::Result<CacheKey<'a>> {
        CacheKey::read_metadata(path)
    }

    /// Gets the cached hash of a file if it exists and is still valid.
    ///
    /// This requires first reading the cache key with [`read_cache_key`](Self::read_cache_key),
    /// which requires accessing the file and can be expensive.
    pub(crate) fn get_cached_hash(
        &self,
        key: &CacheKey,
    ) -> anyhow::Result<Option<(Cow<'static, str>, [u8; 16])>> {
        // check for cached hash
        let cached = {
            let db = self.db.lock().unwrap();
            db.get_file_hash_by_path(key.path)?
        };

        // check if cached hash matches current metadata
        if let Some(cached) = cached {
            if key.matches_file_hash(&cached) {
                return Ok(Some((cached.hash_kind.into(), cached.hash)));
            }
        }

        // otherwise, return None without computing a new hash
        Ok(None)
    }

    /// Gets the hash of a file, computing it if necessary.
    pub fn get_hash(&self, path: &Path) -> anyhow::Result<(Cow<'static, str>, [u8; 16])> {
        // get file metadata
        let key = CacheKey::read_metadata(path)?;

        // check for cached hash
        let cached = {
            let db = self.db.lock().unwrap();
            db.get_file_hash_by_path(path)?
        };

        // check if cached hash matches current metadata
        if let Some(cached) = cached {
            if key.matches_file_hash(&cached) {
                return Ok((cached.hash_kind.into(), cached.hash));
            }
        }

        // get new hash
        let (hash_kind, hash) = musicopy_transcode::hash::get_file_hash(path)?;

        // store new hash
        {
            let db = self.db.lock().unwrap();
            db.insert_file_hash(InsertFileHash {
                path: path.to_string_lossy(),
                last_file_size: key.file_size,
                last_modified_at: key.modified_at,
                hash_kind,
                hash,
            })?;
        }

        Ok((hash_kind.into(), hash))
    }

    /// Gets a set of hashes for multiple files, computing them if necessary.
    ///
    /// The return value is unordered and doesn't correspond with the input.
    pub fn batch_get_hash(
        &self,
        paths: Vec<PathBuf>,
    ) -> anyhow::Result<HashSet<(Cow<'static, str>, [u8; 16])>> {
        // get cached hashes
        let cached = {
            let db = self.db.lock().unwrap();
            db.get_file_hashes_by_paths(paths.iter().map(|p| p.to_string_lossy()))?
        };

        let mut results: Vec<Option<((Cow<'static, str>, [u8; 16]), Option<InsertFileHash>)>> =
            Vec::new();
        paths
            .par_iter()
            .map(|path| {
                // get file metadata
                let key = match CacheKey::read_metadata(path) {
                    Ok(key) => key,
                    Err(e) => {
                        warn!(
                            "failed to read metadata for file: {}: {:#}",
                            path.display(),
                            e
                        );
                        return None;
                    }
                };

                // check if cached hash matches current metadata
                if let Some(cached) = cached.get(path.to_string_lossy().as_ref()) {
                    if key.matches_file_hash(cached) {
                        let hash = (cached.hash_kind.clone().into(), cached.hash);
                        return Some((hash, None));
                    }
                }

                // get new hash
                let (hash_kind, hash) = match musicopy_transcode::hash::get_file_hash(path) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("failed to get hash for file: {}: {:#}", path.display(), e);
                        return None;
                    }
                };

                let insert = InsertFileHash {
                    path: path.to_string_lossy(),
                    last_file_size: key.file_size,
                    last_modified_at: key.modified_at,
                    hash_kind,
                    hash,
                };

                let hash = (hash_kind.into(), hash);
                Some((hash, Some(insert)))
            })
            .collect_into_vec(&mut results);

        let (all_hashes, insert_hashes): (_, Vec<Option<InsertFileHash>>) =
            results.into_iter().flatten().unzip();

        // store new hashes
        {
            let mut db = self.db.lock().unwrap();
            db.insert_file_hashes(insert_hashes.into_iter().flatten())
                .context("failed to insert file hashes")?;
        }

        Ok(all_hashes)
    }

    /// Gets the cached duration of a file if it exists and is still valid.
    ///
    /// This requires first reading the cache key with [`read_cache_key`](Self::read_cache_key),
    /// which requires accessing the file and can be expensive.
    pub(crate) fn get_cached_duration(&self, key: &CacheKey) -> anyhow::Result<Option<f64>> {
        // check for cached duration
        let cached = {
            let db = self.db.lock().unwrap();
            db.get_file_size_by_path(key.path)?
        };

        // check if cached duration matches current metadata
        if let Some(cached) = cached {
            if key.matches_file_size(&cached) {
                return Ok(Some(cached.duration));
            }
        }

        Ok(None)
    }

    /// Cheaply gets the cached estimated size of a file from cache without validating it.
    ///
    /// This does not require reading the cache key first, which requires accessing the file and can
    /// be expensive. This should be used if using the stale duration is allowable and needs to be
    /// fast. Files are unlikely to be modified, and most modifications only change metadata which
    /// has less effect on duration than replacing the audio data, so this will often be correct
    /// anyway.
    pub fn get_cached_duration_unvalidated(&self, path: &Path) -> anyhow::Result<Option<f64>> {
        let db = self.db.lock().unwrap();
        Ok(db
            .get_file_size_by_path(path)?
            .map(|cached| cached.duration))
    }

    /// Cheaply gets the size of a file from cache without validating it.
    ///
    /// This does not require reading the cache key first, which requires accessing the file and can
    /// be expensive. This should be used if using the stale size is allowable and needs to be fast.
    /// This reuses cached durations to get the original file sizes, to provide file sizes when
    /// transferring originals. Sort of a hack... but it's faster than reading the files and we want
    /// the index to prepare quickly.
    pub fn get_cached_file_size_unvalidated(&self, path: &Path) -> anyhow::Result<Option<u64>> {
        let db = self.db.lock().unwrap();
        Ok(db
            .get_file_size_by_path(path)?
            .map(|cached| cached.last_file_size))
    }

    /// Prepares durations for multiple files.
    pub fn batch_get_durations(&self, paths: Vec<PathBuf>) -> anyhow::Result<()> {
        // get cached durations
        let cached = {
            let db = self.db.lock().unwrap();
            db.get_file_sizes_by_paths(paths.iter().map(|p| p.to_string_lossy()))?
        };

        let mut insert_sizes = Vec::new();
        paths
            .par_iter()
            .map(|path| {
                // get file metadata
                let key = match CacheKey::read_metadata(path) {
                    Ok(key) => key,
                    Err(e) => {
                        warn!(
                            "failed to read metadata for file: {}: {:#}",
                            path.display(),
                            e
                        );
                        return None;
                    }
                };

                // check if cached size matches current metadata
                if let Some(cached) = cached.get(path.to_string_lossy().as_ref()) {
                    if key.matches_file_size(cached) {
                        return None;
                    }
                }

                // get new duration
                let duration = match musicopy_transcode::hash::get_file_duration(path) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(
                            "failed to get file duration for {}: {:#}",
                            path.display(),
                            e
                        );
                        return None;
                    }
                };

                Some(InsertFileSize {
                    path: path.to_string_lossy(),
                    last_file_size: key.file_size,
                    last_modified_at: key.modified_at,
                    duration,
                })
            })
            .collect_into_vec(&mut insert_sizes);

        // store new sizes
        {
            let mut db = self.db.lock().unwrap();
            db.insert_file_sizes(insert_sizes.into_iter().flatten())
                .context("failed to insert file sizes")?;
        }

        Ok(())
    }
}
