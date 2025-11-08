use crate::database::{Database, InsertFileHash};
use anyhow::Context;
use std::{
    borrow::Cow,
    hash::Hasher,
    path::Path,
    sync::{Arc, Mutex},
    time::SystemTime,
};
use symphonia::core::{
    codecs::audio::VerificationCheck,
    formats::{TrackType, probe::Hint},
    io::MediaSourceStream,
};
use twox_hash::XxHash3_64;

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
        let metadata = std::fs::metadata(path).context("failed to get file metadata")?;
        let modified_at = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // check for cached hash
        let cached = {
            let db = self.db.lock().unwrap();
            db.get_file_hash_by_path(path)?
        };

        // check if cached hash has the same size and modification time
        if let Some(cached) = cached {
            if cached.last_file_size == metadata.len() && cached.last_modified_at == modified_at {
                return Ok(Some((cached.hash_kind.into(), cached.hash)));
            }
        }

        // otherwise, return None without computing a new hash
        Ok(None)
    }

    /// Gets the hash of a file, computing it if necessary.
    pub fn get_hash(&self, path: &Path) -> anyhow::Result<(Cow<'static, str>, [u8; 16])> {
        // get file metadata
        let metadata = std::fs::metadata(path).context("failed to get file metadata")?;
        let modified_at = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // check for cached hash
        let cached = {
            let db = self.db.lock().unwrap();
            db.get_file_hash_by_path(path)?
        };

        // check if cached hash has the same size and modification time
        if let Some(cached) = cached {
            if cached.last_file_size == metadata.len() && cached.last_modified_at == modified_at {
                return Ok((cached.hash_kind.into(), cached.hash));
            }
        }

        // get new hash
        let (hash_kind, hash) = get_file_hash(path)?;

        // store new hash
        {
            let db = self.db.lock().unwrap();
            db.insert_file_hash(InsertFileHash {
                path: &path.to_string_lossy(),
                last_file_size: metadata.len(),
                last_modified_at: modified_at,
                hash_kind,
                hash,
            })?;
        }

        Ok((hash_kind.into(), hash))
    }
}

/// Get the hash of a file.
///
/// If the file contains an MD5 checksum (many flacs do), then it will be used.
/// Otherwise, the file will be decoded and the audio data will be hashed using
/// xxhash3 with 64-bit hashes, padded to 128 bits to be the same length as MD5.
// TODO: we maybe should also get the estimated file size here
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
