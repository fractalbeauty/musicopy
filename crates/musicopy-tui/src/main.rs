mod app;
mod event;
mod ui;

use crate::{
    app::{App, AppEvent},
    event::app_send,
};
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    /// Whether to store state in memory only, without persisting to disk.
    #[arg(long, short = 'm', default_value_t = false)]
    in_memory: bool,

    /// Whether to automatically accept incoming connections.
    #[arg(long, default_value_t = false)]
    auto_accept: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // initialize app
    let app = App::new(args.in_memory, args.auto_accept).await?;

    // set up global logger
    let logger = AppLogger::new_with_default("warn,musicopy_tui=debug,musicopy=debug");
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(log::LevelFilter::Debug);

    // run tui
    let terminal = ratatui::init();
    let app_result = app.run(terminal).await;
    ratatui::restore();
    app_result
}

/// Logger implementation that logs to the TUI.
struct AppLogger {
    filter: env_filter::Filter,
}

impl AppLogger {
    fn new_with_default(default: &str) -> Self {
        let mut filter_builder = env_filter::Builder::new();
        if let Ok(filter) = &std::env::var("RUST_LOG") {
            filter_builder.parse(filter);
        } else {
            filter_builder.parse(default);
        }
        Self {
            filter: filter_builder.build(),
        }
    }
}

impl log::Log for AppLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.filter.matches(record) {
            let s = record.args().to_string();
            app_send!(AppEvent::Log(s));
        }
    }

    fn flush(&self) {}
}
