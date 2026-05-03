use musicopy_transcode::hash::{get_file_duration, get_file_hash};
use std::{path::Path, process};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .without_time()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        error!("usage: hash <file>");
        process::exit(1);
    }

    let path = Path::new(&args[1]);

    let mut failed = false;

    match get_file_hash(path) {
        Ok((kind, hash)) => info!(
            "get_file_hash: {kind} {}",
            hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ),
        Err(e) => {
            error!("error getting hash: {e:#}");
            failed = true;
        }
    }

    match get_file_duration(path) {
        Ok(duration) => info!("get_file_duration: {duration:.3}s"),
        Err(e) => {
            error!("error getting duration: {e:#}");
            failed = true;
        }
    }

    if failed {
        process::exit(1);
    }
}
