mod app;
mod event;
mod ui;

use crate::{
    app::{App, AppEvent, LogMessage},
    event::app_send,
};
use clap::Parser;
use tracing_subscriber::{EnvFilter, Registry, prelude::*};

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

    // Forward `log` records to `tracing`
    let _ = tracing_log::LogTracer::init();

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,musicopy_tui=debug,musicopy=debug"));

    tracing::subscriber::set_global_default(Registry::default().with(filter).with(TuiLayer))?;

    // initialize app
    let app = App::new(args.in_memory, args.auto_accept).await?;

    // run tui
    let terminal = ratatui::init();
    let app_result = app.run(terminal).await;
    ratatui::restore();
    app_result
}

/// Tracing layer that forwards log events to the TUI.
struct TuiLayer;

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for TuiLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = *metadata.level();
        let target = metadata.target().to_string();

        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);

        app_send!(AppEvent::Log(LogMessage {
            level,
            target,
            message: visitor.0,
        }));
    }
}

struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        } else if self.0.is_empty() {
            self.0 = format!("{} = {value:?}", field.name());
        } else {
            self.0 = format!("{}, {} = {value:?}", self.0, field.name());
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        } else {
            self.record_debug(field, &value);
        }
    }
}
