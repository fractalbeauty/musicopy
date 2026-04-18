use musicopy_transcode::{Mp3Preset, OpusPreset, TranscodePreset, transcode};
use std::{path::Path, process};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 || args.len() > 4 {
        eprintln!("usage: transcode <input> <output> [format]");
        eprintln!("  format: opus128 (default), opus64, mp3v0, mp3v5");
        process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    let format_str = args.get(3).map(String::as_str).unwrap_or("opus128");
    let format = match parse_format(format_str) {
        Ok(format) => format,
        Err(error) => {
            eprintln!("error: {error}");
            process::exit(1);
        }
    };

    match transcode(format, input_path, output_path) {
        Ok(file_size) => println!(
            "transcoded to {} ({file_size} bytes)",
            output_path.display()
        ),
        Err(error) => {
            eprintln!("error: {error:#}");
            process::exit(1);
        }
    }
}

fn parse_format(s: &str) -> Result<TranscodePreset, String> {
    match s {
        "opus128" => Ok(TranscodePreset::Opus(OpusPreset::Opus128)),
        "opus64" => Ok(TranscodePreset::Opus(OpusPreset::Opus64)),
        "mp3v0" => Ok(TranscodePreset::Mp3(Mp3Preset::Mp3V0)),
        "mp3v5" => Ok(TranscodePreset::Mp3(Mp3Preset::Mp3V5)),
        other => Err(format!(
            "unknown format '{other}' (expected: opus128, opus64, mp3v0, mp3v5)"
        )),
    }
}
