use musicopy_transcode::{Mp3Preset, OpusPreset, TranscodePreset, transcode};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Returns all fixture files in `fixtures/`.
fn fixture_files() -> Vec<PathBuf> {
    let mut fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixtures_dir.pop(); // crates/
    fixtures_dir.pop(); // workspace root
    fixtures_dir.push("fixtures");
    assert!(
        fixtures_dir.exists(),
        "fixtures directory does not exist: {}",
        fixtures_dir.display()
    );

    let mut files: Vec<PathBuf> = fs::read_dir(&fixtures_dir)
        .unwrap_or_else(|e| {
            panic!(
                "failed to read fixtures dir {}: {e}",
                fixtures_dir.display()
            )
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.path())
        .filter(|p| {
            // Filter out files starting with . (like `.DS_Store`)
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| !n.starts_with('.'))
        })
        .collect();

    files.sort();

    assert!(
        !files.is_empty(),
        "no fixture files found in {}",
        fixtures_dir.display()
    );

    files
}

fn check(preset: TranscodePreset, input: &Path) {
    let output_dir = testdir::testdir!();
    let stem = input
        .file_stem()
        .expect("fixture has a file stem")
        .to_string_lossy()
        .to_string();
    let extension = match preset {
        TranscodePreset::Opus(_) => "ogg",
        TranscodePreset::Mp3(_) => "mp3",
    };
    let output_path = output_dir.join(format!("{stem}.{extension}"));

    let file_size = transcode(preset, input, &output_path)
        .unwrap_or_else(|e| panic!("transcode failed for {}: {e:#}", input.display()));

    assert!(
        output_path.exists(),
        "output file was not created: {}",
        output_path.display()
    );
    assert!(
        file_size > 0,
        "transcode reported zero bytes for {}",
        input.display()
    );

    let actual_size = fs::metadata(&output_path)
        .unwrap_or_else(|e| panic!("failed to stat output {}: {e}", output_path.display()))
        .len();
    assert_eq!(
        actual_size,
        file_size,
        "reported file size does not match actual size for {}",
        input.display()
    );
}

#[test]
fn transcode_opus128() {
    for file in fixture_files() {
        check(TranscodePreset::Opus(OpusPreset::Opus128), &file);
    }
}

#[test]
fn transcode_opus64() {
    for file in fixture_files() {
        check(TranscodePreset::Opus(OpusPreset::Opus64), &file);
    }
}

#[test]
fn transcode_mp3v0() {
    for file in fixture_files() {
        check(TranscodePreset::Mp3(Mp3Preset::Mp3V0), &file);
    }
}

#[test]
fn transcode_mp3v5() {
    for file in fixture_files() {
        check(TranscodePreset::Mp3(Mp3Preset::Mp3V5), &file);
    }
}
