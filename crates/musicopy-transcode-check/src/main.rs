use anyhow::Context;
use clap::Parser;
use musicopy_transcode::{Mp3Preset, OpusPreset, TranscodePreset, transcode};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .without_time()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    if let Err(e) = run() {
        error!("{e:#}");
        std::process::exit(1);
    }
}

/// Transcodes files in `input` to `output` in all formats and logs successes/errors to `log`.
#[derive(Parser)]
struct Args {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    log: PathBuf,
    /// The maximum number of transcodes to do.
    #[arg(long)]
    count: Option<usize>,
}

fn run() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    if !args.input.is_dir() {
        anyhow::bail!("not a directory: {}", args.input.display());
    }

    fs::create_dir_all(&args.output).context("failed to create output directory")?;

    let mut remaining = find_remaining_audio_files(&args.input, &args.log)?;
    if remaining.is_empty() {
        info!("all (file, format) pairs already processed");
        return Ok(());
    }

    info!("{} (file, format) pairs to transcode", remaining.len());

    remaining.shuffle(&mut rand::thread_rng());

    let count = match args.count {
        Some(n) => n.min(remaining.len()),
        None => remaining.len(),
    };

    for (i, (input_path, preset)) in remaining[..count].iter().enumerate() {
        let hash = {
            let mut hasher = Sha256::new();
            hasher.update(input_path.to_string_lossy().as_bytes());
            hex::encode(hasher.finalize())
        };
        let extension = match preset {
            TranscodePreset::Opus(_) => "ogg",
            TranscodePreset::Mp3(_) => "mp3",
        };
        let output_path = args
            .output
            .join(preset_to_str(*preset))
            .join(format!("{hash}.{extension}",));

        if let Some(parent) = output_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                warn!("failed to create {}: {e}", parent.display());
            }
        }

        info!(
            "[{}/{}] transcoding {} for {}",
            i + 1,
            count,
            preset_to_str(*preset),
            input_path.display(),
        );

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let status = match transcode(*preset, input_path, &output_path) {
            Ok(size) => {
                info!("  -> {} ({size} bytes)", output_path.display());
                StateEntryStatus::Success
            }
            Err(e) => {
                let message = format!("{e:#}");
                error!("  -> error: {message}");
                StateEntryStatus::Error { message }
            }
        };

        let entry = StateEntry {
            input_path: input_path.clone(),
            format: preset_to_str(*preset).to_string(),
            status,
            timestamp,
        };

        if let Err(e) = append_entry(&args.log, &entry) {
            warn!("failed to append state: {e}");
        }
    }

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct StateEntry {
    input_path: PathBuf,
    format: String,
    #[serde(flatten)]
    status: StateEntryStatus,
    timestamp: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum StateEntryStatus {
    Success,
    Error { message: String },
}

/// Reads state entries stored as JSON lines from `path`.
///
/// Returns the set of (file, format) pairs that have already been processed.
fn read_log(path: &Path) -> Result<HashSet<(PathBuf, String)>, anyhow::Error> {
    if !path.exists() {
        return Ok(HashSet::new());
    }

    let file = File::open(path).context("failed to open state file")?;
    let reader = BufReader::new(file);

    let mut pairs = HashSet::new();
    for line in reader.lines() {
        let line = line.context("failed to read state line")?;
        if line.trim().is_empty() {
            continue;
        }

        let entry: StateEntry =
            serde_json::from_str(&line).context("failed to deserialize state line")?;
        pairs.insert((entry.input_path, entry.format));
    }

    Ok(pairs)
}

/// Append a state entry as a JSON line to `path`.
fn append_entry(path: &Path, entry: &StateEntry) -> Result<(), anyhow::Error> {
    let line = serde_json::to_string(entry).context("failed to serialize state entry")?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .context("failed to open state file")?;
    writeln!(file, "{line}").context("failed to append state line")
}

/// Finds (file, format) pairs in `root_dir` that haven't been processed yet.
fn find_remaining_audio_files(
    root_dir: &Path,
    state_file: &Path,
) -> Result<Vec<(PathBuf, TranscodePreset)>, anyhow::Error> {
    let processed_files = read_log(state_file)?;
    let all_files = find_audio_files(root_dir);

    Ok(all_files
        .iter()
        .flat_map(|file| {
            TranscodePreset::ALL
                .iter()
                .map(|&preset| (file.clone(), preset))
        })
        .filter(|(p, preset)| {
            !processed_files.contains(&(p.clone(), preset_to_str(*preset).to_string()))
        })
        .collect())
}

/// Finds audio files in `root_dir`.
fn find_audio_files(root_dir: &Path) -> Vec<PathBuf> {
    let mut output = Vec::new();
    let mut frontier = vec![root_dir.to_path_buf()];

    while let Some(dir) = frontier.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                warn!("failed to read {}: {error}", dir.display());
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                frontier.push(path);
            } else if is_audio_file(&path) {
                output.push(path);
            }
        }
    }

    output
}

/// Checks whether a file is an audio file by checking the extension.
fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| AUDIO_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

const AUDIO_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "ogg", "opus", "m4a", "aac", "wav", "aiff", "aif",
];

fn preset_to_str(preset: TranscodePreset) -> &'static str {
    match preset {
        TranscodePreset::Opus(OpusPreset::Opus128) => "opus128",
        TranscodePreset::Opus(OpusPreset::Opus64) => "opus64",
        TranscodePreset::Mp3(Mp3Preset::Mp3V0) => "mp3v0",
        TranscodePreset::Mp3(Mp3Preset::Mp3V5) => "mp3v5",
    }
}
