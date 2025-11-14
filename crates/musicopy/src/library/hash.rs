use crate::database::{Database, FileHash, FileSize, InsertFileHash, InsertFileSize};
use anyhow::Context;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{
    borrow::Cow,
    collections::HashSet,
    hash::Hasher,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};
use symphonia::core::{
    codecs::audio::VerificationCheck,
    formats::{TrackType, probe::Hint},
    io::MediaSourceStream,
};
use twox_hash::XxHash3_64;

struct CacheKey {
    file_size: u64,
    modified_at: u64,
}

impl CacheKey {
    fn read_metadata(path: &Path) -> anyhow::Result<Self> {
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
        })
    }

    fn matches_file_hash(&self, file_hash: &FileHash) -> bool {
        self.file_size == file_hash.last_file_size && self.modified_at == file_hash.last_modified_at
    }

    fn matches_file_size(&self, file_size: &FileSize) -> bool {
        self.file_size == file_size.last_file_size && self.modified_at == file_size.last_modified_at
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

    /// Cheaply gets the cached hash of a file if it exists and is valid.
    pub fn get_cached_hash(
        &self,
        path: &Path,
    ) -> anyhow::Result<Option<(Cow<'static, str>, [u8; 16])>> {
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
        let (hash_kind, hash) = get_file_hash(path)?;

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
                        log::warn!(
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
                let (hash_kind, hash) = match get_file_hash(path) {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!("failed to get hash for file: {}: {:#}", path.display(), e);
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

    /// Cheaply gets the estimated size of a file if it exists and is valid.
    pub fn get_cached_estimated_size(&self, path: &Path) -> anyhow::Result<Option<u64>> {
        // get file metadata
        let key = CacheKey::read_metadata(path)?;

        // check for cached size
        let cached = {
            let db = self.db.lock().unwrap();
            db.get_file_size_by_path(path)?
        };

        // check if cached size matches current metadata
        if let Some(cached) = cached {
            if key.matches_file_size(&cached) {
                return Ok(Some(cached.estimated_size));
            }
        }

        Ok(None)
    }

    /// Prepares estimated sizes for multiple files.
    pub fn batch_get_estimated_size(&self, paths: Vec<PathBuf>) -> anyhow::Result<()> {
        // get cached sizes
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
                        log::warn!(
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

                // get new size
                let (duration, estimated_size) = match estimate_file_size(path) {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!(
                            "failed to estimate file size for {}: {:#}",
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
                    estimated_size,
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

/// Get the hash of a file.
///
/// If the file contains an MD5 checksum (many flacs do), then it will be used.
/// Otherwise, the file will be decoded and the audio data will be hashed using
/// xxhash3 with 64-bit hashes, padded to 128 bits to be the same length as MD5.
fn get_file_hash(path: &Path) -> anyhow::Result<(&'static str, [u8; 16])> {
    let src = std::fs::File::open(path).context("failed to open file")?;

    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(extension) = path.extension() {
        hint.with_extension(extension.to_str().context("invalid file extension")?);
    }

    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, Default::default(), Default::default())
        .context("failed to probe file")?;

    // get the default audio track
    let audio_track = format
        .default_track(TrackType::Audio)
        .context("failed to get default audio track")?;
    let audio_track_id = audio_track.id;

    // check if MD5 verification check is available (common for flacs)
    if let Some(VerificationCheck::Md5(verification_md5)) = &audio_track
        .codec_params
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("failed to get codec params"))?
        .audio()
        .ok_or_else(|| anyhow::anyhow!("failed to get audio codec params"))?
        .verification_check
    {
        Ok(("md5", *verification_md5))
    } else {
        let mut hasher = XxHash3_64::with_seed(8888);

        loop {
            // read next packet
            let packet = match format.next_packet() {
                Ok(Some(packet)) => packet,

                // end of track
                Ok(None) => break,

                Err(e) => anyhow::bail!("failed to read packet: {e}"),
            };

            // skip packets from other tracks
            if packet.track_id() != audio_track_id {
                continue;
            }

            // hash the packet bytes, without decoding them.
            // this is maybe more stable than hashing the decoded samples, and
            // should still stay the same when metadata is modified.
            hasher.write(packet.buf());
        }

        // the convention for xxhash is to use big-endian byte order
        // https://github.com/Cyan4973/xxHash/blob/55d9c43608e39b2acd7d9a9cc3df424f812b6642/xxhash.h#L192
        let hash = (hasher.finish() as u128).to_be_bytes();

        Ok(("xxh3", hash))
    }
}

/// Estimates the size of a file after transcoding based on its duration.
///
/// Returns (duration in seconds, estimated size in bytes).
fn estimate_file_size(path: &Path) -> anyhow::Result<(f64, u64)> {
    let src = std::fs::File::open(path).context("failed to open file")?;

    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(extension) = path.extension() {
        hint.with_extension(extension.to_str().context("invalid file extension")?);
    }

    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, Default::default(), Default::default())
        .context("failed to probe file")?;

    // get the default audio track
    let audio_track = format
        .default_track(TrackType::Audio)
        .context("failed to get default audio track")?;
    let audio_track_id = audio_track.id;

    // get time base and number of frames from the audio track
    let duration_secs = match (audio_track.time_base, audio_track.num_frames) {
        (Some(time_base), Some(num_frames)) => {
            let duration = time_base.calc_time(num_frames);
            duration.seconds as f64 + duration.frac
        }

        // TODO: check if this actually happens in practice. it is probably slow to decode the whole file
        _ => {
            log::warn!(
                "file missing time_base or num_frames, decoding to find duration: {}",
                path.display()
            );

            // get codec parameters for the audio track
            let codec_params = audio_track
                .codec_params
                .as_ref()
                .context("failed to get codec parameters")?;
            let audio_codec_params = codec_params
                .audio()
                .context("codec parameters are not audio")?;

            // get sample rate
            let sample_rate = audio_codec_params
                .sample_rate
                .context("failed to get sample rate from codec params")?;

            let mut decoder = symphonia::default::get_codecs()
                .make_audio_decoder(audio_codec_params, &Default::default())
                .context("failed to create decoder")?;

            // decode the audio track and count frames
            let mut num_frames = 0;
            loop {
                // read next packet
                let packet = match format.next_packet() {
                    Ok(Some(packet)) => packet,

                    // end of track
                    Ok(None) => break,

                    Err(e) => {
                        return Err(e).context("failed to read packet");
                    }
                };

                // skip packets from other tracks
                if packet.track_id() != audio_track_id {
                    continue;
                }

                // decode packet
                let audio_buf = decoder.decode(&packet).context("failed to decode packet")?;

                // count frames
                num_frames += audio_buf.frames();
            }

            // convert frames to seconds
            num_frames as f64 / sample_rate as f64
        }
    };

    // estimated size = duration * bitrate (128k), converted to bytes
    let estimated_size = duration_secs * 128_000.0 / 8.0;

    // add 150 KB for embedded cover art
    let estimated_size = estimated_size + 150_000.0;

    // add 1% for container overhead
    let estimated_size = estimated_size * 1.01;

    Ok((duration_secs, estimated_size as u64))
}
