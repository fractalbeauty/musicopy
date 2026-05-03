use anyhow::Context;
use std::{hash::Hasher, path::Path};

#[cfg(feature = "transcode")]
use symphonia::core::{
    codecs::audio::VerificationCheck,
    formats::{Track, TrackType, probe::Hint},
    io::MediaSourceStream,
    units::Timestamp,
};
#[cfg(feature = "transcode")]
use tracing::{trace, warn};
#[cfg(feature = "transcode")]
use twox_hash::XxHash3_64;

/// Get the hash of a file.
///
/// If the file contains an MD5 checksum (many flacs do), then it will be used.
/// Otherwise, the file will be decoded and the audio data will be hashed using
/// xxhash3 with 64-bit hashes, padded to 128 bits to be the same length as MD5.
#[cfg(feature = "transcode")]
pub fn get_file_hash(path: &Path) -> anyhow::Result<(&'static str, [u8; 16])> {
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

/// Gets the duration of a file in seconds by reading its metadata or decoding it if necessary.
#[cfg(feature = "transcode")]
pub fn get_file_duration(path: &Path) -> anyhow::Result<f64> {
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

    // get time base and duration from the audio track
    trace!(
        "audio_track has: time_base? {}, duration? {}, num_frames? {}",
        audio_track.time_base.is_some(),
        audio_track.duration.is_some(),
        audio_track.num_frames.is_some()
    );
    let duration_secs = if let Some(duration) = get_audio_track_duration(&audio_track) {
        duration
    } else {
        // TODO: check if this actually happens in practice. it is probably slow to decode the whole file
        warn!(
            "file missing time_base or num_frames/duration, decoding to find duration: {}",
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
    };

    Ok(duration_secs)
}

/// Get the duration in seconds from an audio track using the time base and num frames or duration.
#[cfg(feature = "transcode")]
fn get_audio_track_duration(audio_track: &Track) -> Option<f64> {
    let Some(time_base) = audio_track.time_base else {
        return None;
    };

    if let Some(num_frames) = audio_track.num_frames {
        let end_timestamp = Timestamp::new(num_frames as i64);
        let time = time_base.calc_time(end_timestamp)?;
        Some(time.as_secs_f64())
    } else if let Some(duration) = audio_track.duration {
        let end_timestamp = duration.timestamp_from(Timestamp::ZERO)?;
        let time = time_base.calc_time(end_timestamp)?;
        Some(time.as_secs_f64())
    } else {
        None
    }
}

/// Stub implementation when compiled without the `transcode` feature.
#[cfg(not(feature = "transcode"))]
pub fn get_file_hash(_path: &Path) -> anyhow::Result<(&'static str, [u8; 16])> {
    anyhow::bail!("get_file_hash is not supported without the transcode feature")
}

/// Stub implementation when compiled without the `transcode` feature.
#[cfg(not(feature = "transcode"))]
pub fn get_file_duration(_path: &Path) -> anyhow::Result<f64> {
    anyhow::bail!("get_file_duration is not supported without the transcode feature")
}
