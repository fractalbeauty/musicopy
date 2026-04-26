use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, Registry, fmt, prelude::*};

/// Initialize logging.
///
/// If `log_dir` is provided, logs will be written to files, rotated at 5 MB per file with 5 files
/// retained and gzip compression. A guard will be returned that should be kept alive until the
/// application exits to flush logs on drop.
///
/// On Android and iOS, logs will also be forwarded to the platform logging systems. On other
/// platforms, logs will also be written to stdout.
pub fn init(log_dir: Option<&Path>) -> anyhow::Result<Option<WorkerGuard>> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,musicopy=debug"));

    let guard = if let Some(log_dir) = log_dir {
        if let Err(e) = std::fs::create_dir_all(log_dir) {
            anyhow::bail!(
                "failed to create log directory at {}: {e:#}",
                log_dir.display()
            );
        }

        let roller = logroller::LogRollerBuilder::new(log_dir, Path::new("musicopy.log"))
            .rotation(logroller::Rotation::SizeBased(logroller::RotationSize::MB(5)))
            .max_keep_files(5)
            .compression(logroller::Compression::Gzip)
            .build()?;

        let (non_blocking, guard) = tracing_appender::non_blocking(roller);

        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            let subscriber = Registry::default()
                .with(filter)
                .with(fmt::Layer::new().with_writer(non_blocking).with_ansi(false))
                .with(fmt::Layer::new().with_writer(std::io::stdout));
            let _ = tracing::subscriber::set_global_default(subscriber);
        }

        #[cfg(target_os = "android")]
        {
            let subscriber = Registry::default()
                .with(filter)
                .with(fmt::Layer::new().with_writer(non_blocking).with_ansi(false))
                .with(
                    tracing_android::layer("musicopy")
                        .expect("failed to init android tracing layer"),
                );
            let _ = tracing::subscriber::set_global_default(subscriber);
        }

        #[cfg(target_os = "ios")]
        {
            let subscriber = Registry::default()
                .with(filter)
                .with(fmt::Layer::new().with_writer(non_blocking).with_ansi(false))
                .with(tracing_oslog::OsLogger::new("app.musicopy", "default"));
            let _ = tracing::subscriber::set_global_default(subscriber);
        }

        Some(guard)
    } else {
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            let subscriber = Registry::default()
                .with(filter)
                .with(fmt::Layer::new().with_writer(std::io::stdout));
            let _ = tracing::subscriber::set_global_default(subscriber);
        }

        #[cfg(target_os = "android")]
        {
            let subscriber = Registry::default().with(filter).with(
                tracing_android::layer("musicopy").expect("failed to init android tracing layer"),
            );
            let _ = tracing::subscriber::set_global_default(subscriber);
        }

        #[cfg(target_os = "ios")]
        {
            let subscriber = Registry::default()
                .with(filter)
                .with(tracing_oslog::OsLogger::new("app.musicopy", "default"));
            let _ = tracing::subscriber::set_global_default(subscriber);
        }

        None
    };

    // Forward `log` records to `tracing`
    let _ = tracing_log::LogTracer::init();

    // Log on panic
    log_panics::init();

    Ok(guard)
}
