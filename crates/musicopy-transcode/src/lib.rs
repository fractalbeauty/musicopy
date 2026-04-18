use anyhow::Context;
use std::{
    fs::File,
    io::{Cursor, Seek, SeekFrom, Write},
    path::Path,
};

#[cfg(feature = "transcode")]
use base64::{Engine, prelude::BASE64_STANDARD};
#[cfg(feature = "transcode")]
use id3::TagLike;
#[cfg(feature = "transcode")]
use image::{ImageReader, codecs::jpeg::JpegEncoder, imageops::FilterType};
#[cfg(feature = "transcode")]
use mp3lame_encoder::{DualPcm, FlushNoGap, MonoPcm};
#[cfg(feature = "transcode")]
use rubato::{FftFixedIn, Resampler};
#[cfg(feature = "transcode")]
use symphonia::core::{
    formats::{FormatReader, TrackType, probe::Hint},
    io::MediaSourceStream,
    meta::{MetadataRevision, StandardTag, StandardVisualKey, Visual},
};

pub enum TranscodePreset {
    Opus(OpusPreset),
    Mp3(Mp3Preset),
}

pub enum OpusPreset {
    Opus128,
    Opus64,
}

pub enum Mp3Preset {
    Mp3V0,
    Mp3V5,
}

/// Transcode a file.
///
/// Returns the file size of the output file.
#[cfg(feature = "transcode")]
pub fn transcode(
    transcode_preset: TranscodePreset,
    input_path: &Path,
    output_path: &Path,
) -> anyhow::Result<u64> {
    let input_file = File::open(input_path).context("failed to open input file")?;

    let mss = MediaSourceStream::new(Box::new(input_file), Default::default());

    let mut hint = Hint::new();
    if let Some(extension) = input_path.extension() {
        hint.with_extension(extension.to_str().context("invalid file extension")?);
    }

    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, Default::default(), Default::default())
        .context("failed to probe file format")?;

    // get the default audio track
    let audio_track = format
        .default_track(TrackType::Audio)
        .context("failed to get default audio track")?;
    let audio_track_id = audio_track.id;

    // get codec parameters for the audio track
    let codec_params = audio_track
        .codec_params
        .as_ref()
        .context("failed to get codec parameters")?;
    let audio_codec_params = codec_params
        .audio()
        .context("codec parameters are not audio")?;

    // get channel count and sample rate from codec parameters
    let channel_count = audio_codec_params
        .channels
        .as_ref()
        .context("failed to get channel count from codec params")?
        .count();
    let sample_rate = audio_codec_params
        .sample_rate
        .context("failed to get sample rate from codec params")? as usize;

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(audio_codec_params, &Default::default())
        .context("failed to create decoder")?;

    // decode the audio track into planar samples
    let mut original_samples: Vec<Vec<f32>> = vec![Vec::new(); channel_count];
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

        // copy to output buffer
        // symphonia only lets us copy to vecs/slices, which replaces instead of extending
        // we need to manually resize each channel and then copy to mut slices of the new extended areas
        let mut output_slices = Vec::with_capacity(channel_count);
        for channel in &mut original_samples {
            let curr_len = channel.len();
            let new_len = curr_len + audio_buf.frames();
            channel.resize(new_len, 0.0);
            output_slices.push(&mut channel[curr_len..new_len]);
        }
        audio_buf.copy_to_slice_planar(&mut output_slices);
    }

    match transcode_preset {
        TranscodePreset::Opus(preset) => transcode_opus(
            preset,
            output_path,
            format,
            channel_count,
            sample_rate,
            original_samples,
        ),
        TranscodePreset::Mp3(preset) => transcode_mp3(
            preset,
            output_path,
            format,
            channel_count,
            sample_rate,
            original_samples,
        ),
    }
}

#[cfg(feature = "transcode")]
fn transcode_opus(
    preset: OpusPreset,
    output_path: &Path,
    mut format: Box<dyn FormatReader>,
    channel_count: usize,
    sample_rate: usize,
    original_samples: Vec<Vec<f32>>,
) -> anyhow::Result<u64> {
    // construct the encoder before resampling to determine the lookahead
    let mut encoder = opus::Encoder::new(
        48000,
        match channel_count {
            1 => opus::Channels::Mono,
            2 => opus::Channels::Stereo,
            _ => anyhow::bail!("unsupported channel count: {}", channel_count),
        },
        opus::Application::Audio,
    )
    .context("failed to create opus encoder")?;
    encoder
        .set_bitrate(match preset {
            OpusPreset::Opus128 => opus::Bitrate::Bits(128000),
            OpusPreset::Opus64 => opus::Bitrate::Bits(64000),
        })
        .context("failed to set opus bitrate")?;

    let lookahead_frames = encoder
        .get_lookahead()
        .context("failed to get opus encoder lookahead")? as usize;

    // resample to 48k if needed
    // also pad the start with zeros to account for encoder lookahead. doing
    // this now allows the encoding logic to be simpler and more efficient.
    let mut resampled_samples = if sample_rate != 48000 {
        let mut resampler = FftFixedIn::<f32>::new(
            sample_rate,
            48000,
            1024, // arbitrary
            4,    // arbitrary
            channel_count,
        )
        .context("failed to create resampler")?;

        let delay = resampler.output_delay();

        let original_frames = original_samples[0].len();

        // number of frames after resampling, including zero-padding for encoder lookahead
        let new_frames = (original_frames * 48000 / sample_rate) + lookahead_frames;

        // pre-allocate output buffer with enough capacity
        // TODO: we might need a little more than this, should check its final capacity to see if it gets resized usually
        let mut resampled_samples: Vec<Vec<f32>> =
            vec![Vec::with_capacity(new_frames + delay); channel_count];

        // pad start with zeros
        for channel in resampled_samples.iter_mut() {
            channel.resize(lookahead_frames, 0.0);
        }

        // allocate chunk input slices vec and chunk output buffer
        let mut input_slices: Vec<&[f32]> = vec![&[]; channel_count];
        let mut output_buf = resampler.output_buffer_allocate(true);

        // resample in chunks
        let mut pos = 0;
        loop {
            // get number of frames needed for next chunk
            let frames_needed = resampler.input_frames_next();

            // check if we have enough frames for a full chunk
            if pos + frames_needed > original_frames {
                break;
            }

            // copy reference to slice of original buffer to input slices vec
            for i in 0..channel_count {
                input_slices[i] = &original_samples[i][pos..(pos + frames_needed)];
            }

            // call resampler with chunk input slices vec and chunk output buffer
            let (input_frames, output_frames) = resampler
                .process_into_buffer(&input_slices, &mut output_buf, None)
                .expect("bad inputs to resampler");

            // copy chunk output buffer to resampled samples
            for i in 0..channel_count {
                resampled_samples[i].extend_from_slice(&output_buf[i][0..output_frames]);
            }

            // increment position by number of input frames consumed
            pos += input_frames;
        }

        // resample final chunk with remaining frames
        if pos < original_frames {
            // copy reference to remaining frames in original samples to input buffer
            for i in 0..channel_count {
                input_slices[i] = &original_samples[i][pos..original_frames];
            }

            let (_input_frames, output_frames) = resampler
                .process_partial_into_buffer(Some(&input_slices), &mut output_buf, None)
                .expect("bad inputs to resampler");

            // copy chunk output buffer to resampled samples
            for i in 0..channel_count {
                resampled_samples[i].extend_from_slice(&output_buf[i][0..output_frames]);
            }
        }

        // continue feeding zeros to the resampler until we have enough frames
        // this ensures we account for resample delay and push everything through its internal buffer
        while resampled_samples[0].len() < new_frames + delay {
            let (_input_frames, output_frames) = resampler
                .process_partial_into_buffer(None::<&[&[f32]]>, &mut output_buf, None)
                .expect("bad inputs to resampler");

            // copy chunk output buffer to resampled samples
            for i in 0..channel_count {
                resampled_samples[i].extend_from_slice(&output_buf[i][0..output_frames]);
            }
        }

        // remove resample delay frames from the start and truncate to new frame count
        // TODO: can we do this without a copy from .drain()?
        for channel in resampled_samples.iter_mut() {
            channel.drain(0..delay);
            channel.truncate(new_frames);
        }

        resampled_samples
    } else {
        // we don't need to resample, but we still need to pad the start with zeros

        let original_frames = original_samples[0].len();

        let mut resampled_samples = vec![Vec::new(); channel_count];
        for i in 0..channel_count {
            resampled_samples[i].resize(lookahead_frames + original_frames, 0.0);
            resampled_samples[i][lookahead_frames..].copy_from_slice(&original_samples[i][..]);
        }

        resampled_samples
    };

    // interleave samples since opus needs interleaved input
    // TODO: profile + explore SIMD for this
    let interleaved_samples = if channel_count == 2 {
        let mut interleaved_samples = vec![0.0; resampled_samples[0].len() * channel_count];

        for i in 0..resampled_samples[0].len() {
            for j in 0..channel_count {
                interleaved_samples[i * channel_count + j] = resampled_samples[j][i];
            }
        }

        interleaved_samples
    } else if channel_count == 1 {
        resampled_samples.swap_remove(0)
    } else {
        unreachable!();
    };

    let mut output_file = File::create(output_path).context("failed to create output file")?;

    let mut packet_writer = ogg::PacketWriter::new(&mut output_file);

    // we write the number of lookahead frames as pre-skip in the opus header
    // we added this many zeros to the start of the resampled samples to account for encoder lookahead
    // players should skip these frames when decoding
    let preskip_bytes = lookahead_frames.to_le_bytes();

    // input sample rate is always 48000 since we resample to it
    let rate_bytes = 48000u32.to_le_bytes();

    #[rustfmt::skip]
	let opus_head: [u8; 19] = [
        b'O', b'p', b'u', b's', b'H', b'e', b'a', b'd', // magic signature
        1, // version, always 1
        channel_count as u8, // channel count
        preskip_bytes[0], preskip_bytes[1], // pre-skip
        rate_bytes[0], rate_bytes[1], rate_bytes[2], rate_bytes[3], // input sample rate
        0, 0, // output gain
        0, // channel mapping family
    ];

    let (user_comments_len, user_comments_buf) = {
        let mut len = 0u32;
        let mut buf = Vec::new();

        if let Some(metadata) = format.metadata().skip_to_latest() {
            for tag in metadata.media.tags.iter().flat_map(|t| &t.std) {
                // TODO: escape = in tag values
                let comment = match tag {
                    StandardTag::TrackTitle(tag) => Some(format!("TITLE={tag}")),
                    StandardTag::Album(tag) => Some(format!("ALBUM={tag}")),
                    StandardTag::TrackNumber(tag) => Some(format!("TRACKNUMBER={tag}")),
                    StandardTag::Artist(tag) => Some(format!("ARTIST={tag}")),
                    _ => None,
                };

                if let Some(s) = comment {
                    len += 1;
                    buf.extend((s.len() as u32).to_le_bytes());
                    buf.extend(s.bytes());
                }
            }

            if let Some(visual) = get_best_visual(metadata) {
                let image_buf =
                    resize_cover_art(&visual.data).context("failed to encode cover art")?;

                // construct flac picture structure
                // note that flac uses big endian while vorbis comments use little endian
                let mut picture = Vec::<u8>::new();
                picture.extend(&3u32.to_be_bytes()); // picture type (3, front cover)

                let media_type = "image/jpeg";
                picture.extend(&(media_type.len() as u32).to_be_bytes());
                picture.extend(media_type.as_bytes());

                picture.extend(&[0, 0, 0, 0]); // description length
                picture.extend(&500u32.to_be_bytes()); // width (500px)
                picture.extend(&500u32.to_be_bytes()); // height (500px)
                picture.extend(&[0, 0, 0, 0]); // color depth (0, unknown)
                picture.extend(&[0, 0, 0, 0]); // indexed color count (0, non-indexed)

                picture.extend(&(image_buf.len() as u32).to_be_bytes()); // picture data length
                picture.extend(&image_buf); // picture data

                // encode picture with base64 for comment
                let comment = format!(
                    "METADATA_BLOCK_PICTURE={}",
                    BASE64_STANDARD.encode(&picture)
                );

                log::debug!(
                    "adding visual to opus tags, image size = {}, comment size = {}",
                    image_buf.len(),
                    comment.len(),
                );

                len += 1;
                buf.extend((comment.len() as u32).to_le_bytes());
                buf.extend(comment.as_bytes());
            }
        }

        (len, buf)
    };

    #[rustfmt::skip]
    let opus_tags = {
        let mut buf = vec![
            b'O', b'p', b'u', b's', b'T', b'a', b'g', b's', // magic signature
            0x08, 0x00, 0x00, 0x00, // vendor string length (8u32 in little-endian)
            b'm', b'u', b's', b'i', b'c', b'o', b'p', b'y', // vendor string
        ];
        buf.extend(user_comments_len.to_le_bytes());
        buf.extend(user_comments_buf);
        buf
    };

    // stream unique serial identifier
    let serial = 0;

    // write opus head and opus tags packets
    packet_writer
        .write_packet(&opus_head, serial, ogg::PacketWriteEndInfo::EndPage, 0)
        .context("failed to write packet")?;
    packet_writer
        .write_packet(&opus_tags, serial, ogg::PacketWriteEndInfo::EndPage, 0)
        .context("failed to write packet")?;

    // number of frames per chunk (48khz / 1000 * 20ms = 960 frames)
    // NB: we are calling opus frames 'chunks' to differentiate from sample frames (one sample per channel)
    let chunk_frames = 48000 / 1000 * 20;
    let chunk_samples = chunk_frames * channel_count;

    let interleaved_len = interleaved_samples.len();

    // encode in chunks
    let mut pos = 0;
    loop {
        // check if we have enough samples for a full chunk
        if pos + chunk_samples > interleaved_len {
            break;
        }

        // allocate chunk output buffer
        // encode_float uses the length (not capacity) as max_data_size
        // length comes from recommended max_data_size in opus documentation
        let mut output_buf = vec![0; 4000];

        // call encoder with input slice and chunk output buffer
        let output_len = encoder
            .encode_float(
                &interleaved_samples[pos..(pos + chunk_samples)],
                &mut output_buf,
            )
            .context("failed to encode chunk")?;
        output_buf.truncate(output_len);

        let end_info = if pos + chunk_samples == interleaved_len {
            // if this chunk ended exactly at the end of input
            ogg::PacketWriteEndInfo::EndStream
        } else {
            ogg::PacketWriteEndInfo::NormalPacket
        };

        // the number of frames up to and including the last frame in this packet
        // this is measured in frames, so mono and stereo increase at the same rate
        let granule_position = ((pos + chunk_samples) / channel_count) as u64;

        // write packet
        packet_writer
            .write_packet(output_buf, serial, end_info, granule_position)
            .context("failed to write packet")?;

        // increment position by number of samples consumed
        pos += chunk_samples;
    }

    // encode final chunk with remaining samples
    if pos < interleaved_len {
        // allocate chunk output buffer
        let mut output_buf = vec![0; 4000];

        // opus always requires a full chunk of input but we don't have enough remaining samples,
        // so allocate a zero-padded input buffer for the final chunk
        let mut input_buf = vec![0.0; chunk_samples];
        input_buf[0..(interleaved_len - pos)]
            .copy_from_slice(&interleaved_samples[pos..interleaved_len]);

        // call encoder with chunk input buffer and chunk output buffer
        let output_len = encoder
            .encode_float(&input_buf, &mut output_buf)
            .context("failed to encode final chunk")?;
        output_buf.truncate(output_len);

        // for end-trimming, the granule position of the final packet is the total number of input frames
        // this may be less than the position of the final frame in the final packet
        // this allows the player to trim the padding samples from the final chunk
        let granule_position = (interleaved_len / channel_count) as u64;

        // write packet
        packet_writer
            .write_packet(
                output_buf,
                serial,
                ogg::PacketWriteEndInfo::EndStream,
                granule_position,
            )
            .context("failed to write packet")?;
    }

    let file = packet_writer.into_inner();
    let file_size = file
        .seek(SeekFrom::End(0))
        .context("failed to seek to end of file")?;

    // we did it
    Ok(file_size)
}

#[cfg(feature = "transcode")]
fn transcode_mp3(
    preset: Mp3Preset,
    output_path: &Path,
    mut format: Box<dyn FormatReader>,
    channel_count: usize,
    sample_rate: usize,
    original_samples: Vec<Vec<f32>>,
) -> anyhow::Result<u64> {
    // extract metadata and build ID3 tags
    let mut tags = id3::Tag::new();
    if let Some(metadata) = format.metadata().skip_to_latest() {
        for tag in metadata.media.tags.iter().flat_map(|t| &t.std) {
            match tag {
                StandardTag::TrackTitle(tag) => tags.set_title(tag.to_string()),
                StandardTag::Artist(tag) => tags.set_artist(tag.to_string()),
                StandardTag::Album(tag) => tags.set_album(tag.to_string()),
                StandardTag::ReleaseDate(tag) | StandardTag::RecordingDate(tag) => {
                    if let Ok(y) = tag.get(..4).unwrap_or("").parse::<i32>() {
                        tags.set_year(y);
                    }
                }
                StandardTag::ReleaseYear(tag) => {
                    tags.set_year((*tag).into());
                }
                StandardTag::TrackNumber(tag) => {
                    if let Ok(tag) = (*tag).try_into() {
                        tags.set_track(tag);
                    }
                }
                _ => {}
            }
        }
        if let Some(visual) = get_best_visual(metadata) {
            let cover_art = resize_cover_art(&visual.data).context("failed to encode cover art")?;
            tags.add_frame(id3::frame::Picture {
                mime_type: "image/jpeg".to_string(),
                picture_type: id3::frame::PictureType::CoverFront,
                description: String::new(),
                data: cover_art,
            });
        }
    }

    let mut output_file = File::create(output_path).context("failed to create output file")?;

    // write ID3 tags and store tag length so we can write the VBR tag after it later
    tags.write_to(&mut output_file, id3::Version::Id3v24)
        .context("failed to write ID3 tags")?;
    let id3_len = output_file
        .stream_position()
        .context("failed to get ID3 tags len")?;

    let mut builder =
        mp3lame_encoder::Builder::new().context("failed to create encoder builder")?;

    // save the inner lame pointer so we can call lame_get_lametag_frame later
    let lame_ptr = unsafe { builder.as_ptr() };

    let mut encoder = builder
        .with_num_channels(channel_count as u8)
        .map_err(|_| anyhow::anyhow!("failed to set channel count"))?
        .with_sample_rate(sample_rate as u32)
        .map_err(|_| anyhow::anyhow!("failed to set sample rate"))?
        .with_vbr_mode(mp3lame_encoder::VbrMode::Mtrh)
        .map_err(|_| anyhow::anyhow!("failed to set vbr mode"))?
        .with_vbr_quality(match preset {
            Mp3Preset::Mp3V0 => mp3lame_encoder::Quality::Best,
            Mp3Preset::Mp3V5 => mp3lame_encoder::Quality::Good,
        })
        .map_err(|_| anyhow::anyhow!("failed to set vbr quality"))?
        .build()
        .map_err(|_| anyhow::anyhow!("failed to build encoder"))?;

    // number of frames per chunk (multiple of 1152 which is the internal frame size)
    let chunk_frames = 4 * 1152;

    // buffer size recommended by lame
    let output_buf_len = chunk_frames * 5 / 4 + 7200;
    let mut output_buf = Vec::with_capacity(output_buf_len);

    // encode and write chunks
    match channel_count {
        1 => {
            for chunk in original_samples[0].chunks(chunk_frames) {
                let output_len = encoder
                    .encode_to_vec(MonoPcm(chunk), &mut output_buf)
                    .map_err(|_| anyhow::anyhow!("failed to encode chunk"))?;

                output_file.write_all(&output_buf[..output_len])?;

                output_buf.clear();
            }
        }
        2 => {
            for (left, right) in original_samples[0]
                .chunks(chunk_frames)
                .zip(original_samples[1].chunks(chunk_frames))
            {
                let output_len = encoder
                    .encode_to_vec(DualPcm { left, right }, &mut output_buf)
                    .map_err(|_| anyhow::anyhow!("failed to encode chunk"))?;

                output_file.write_all(&output_buf[..output_len])?;

                output_buf.clear();
            }
        }
        _ => anyhow::bail!("unsupported channel count: {}", channel_count),
    }

    // flush encoder and write
    let output_len = encoder
        .flush_to_vec::<FlushNoGap>(&mut output_buf)
        .map_err(|_| anyhow::anyhow!("failed to flush encoder"))?;
    output_file.write_all(&output_buf[..output_len])?;

    // store final file size
    let file_size = output_file
        .stream_position()
        .context("failed to get file length")?;

    // call lame_get_lametag_frame to get VBR tag and write it just after the ID3 tags
    let res = unsafe {
        mp3lame_encoder::ffi::lame_get_lametag_frame(
            lame_ptr,
            output_buf.as_mut_ptr(),
            output_buf.capacity(),
        )
    };
    if res > output_buf.capacity() {
        anyhow::bail!("buffer too small for lame_get_lametag_frame");
    }
    unsafe {
        output_buf.set_len(res);
    }
    output_file
        .seek(SeekFrom::Start(id3_len))
        .context("failed to seek to VBR tag position")?;
    output_file
        .write_all(&output_buf[..res])
        .context("failed to write VBR tag")?;

    // we did it 2
    Ok(file_size)
}

/// Find the front cover visual or the first available.
#[cfg(feature = "transcode")]
fn get_best_visual(metadata: &MetadataRevision) -> Option<&Visual> {
    let mut best_visual = metadata.media.visuals.first();
    for visual in &metadata.media.visuals {
        if visual.usage == Some(StandardVisualKey::FrontCover) {
            best_visual = Some(visual);
        }
    }
    best_visual
}

/// Convert cover art to a 500x500 JPEG with 90% quality.
#[cfg(feature = "transcode")]
fn resize_cover_art(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let rdr = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .expect("cursor io never fails");
    let original_image = rdr.decode().context("failed to decode image")?;

    let resized_image = original_image.resize(500, 500, FilterType::Lanczos3);

    let mut image_buf = vec![];
    JpegEncoder::new_with_quality(&mut image_buf, 90)
        .encode_image(&resized_image)
        .context("failed to encode image")?;

    Ok(image_buf)
}

/// Stub implementation when compiled without the `transcode` feature.
#[cfg(not(feature = "transcode"))]
pub fn transcode(
    _transcode_preset: TranscodePreset,
    _input_path: &Path,
    _output_path: &Path,
) -> anyhow::Result<u64> {
    anyhow::bail!("transcoding is not supported without the transcode feature")
}
