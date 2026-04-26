use crate::{
    event::{Event, EventHandler, app_send},
    ui::log::LogState,
};
use anyhow::Context;
use musicopy::{
    Core, CoreOptions, StatsModel,
    library::{LibraryModel, transcode::TranscodeFormat},
    node::{ClientStateModel, DownloadRequestModel, NodeModel, ServerStateModel},
};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};
use std::{sync::Arc, time::SystemTime};
use tracing::{error, info};
use tui_widgets::prompts::{State, Status, TextState};

/// Application.
#[derive(Debug)]
pub struct App<'a> {
    pub running: bool,
    pub events: EventHandler,

    pub core: Arc<Core>,

    pub mode: AppMode,
    pub screen: AppScreen,

    pub messages: Vec<LogMessage>,

    pub log_state: LogState,
    pub command_state: TextState<'a>,

    pub library_model: LibraryModel,
    pub node_model: NodeModel,
    pub stats_model: StatsModel,

    pub transcode_format: Option<TranscodeFormat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppScreen {
    #[default]
    Home,
    Log,
    Stats,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    Default,
    Command,
}

/// Application events.
#[derive(Debug)]
pub enum AppEvent {
    Log(LogMessage),

    Exit,

    CommandMode,
    ExitMode,

    Screen(AppScreen),
    LibraryModel(Box<LibraryModel>),
    NodeModel(Box<NodeModel>),
    StatsModel(Box<StatsModel>),
}

#[derive(Debug)]
pub struct LogMessage {
    pub level: tracing::Level,
    pub target: String,
    pub message: String,
}

impl<'a> App<'a> {
    /// Constructs a new instance of [`App`].
    pub async fn new(in_memory: bool, auto_accept: bool) -> anyhow::Result<Self> {
        // initialize as early as possible
        let events = EventHandler::new();

        let core = Core::start(
            Arc::new(AppEventHandler),
            CoreOptions {
                init_logging: false,
                in_memory,
                project_dirs: None,
            },
        )
        .await?;

        // spawn auto accept task
        if auto_accept {
            info!("ran with --auto-accept, will automatically accept incoming connections");
            let core = core.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    let node_model = match core.get_node_model() {
                        Ok(model) => model,
                        Err(e) => {
                            error!("auto accept: error getting node model: {e:#}");
                            continue;
                        }
                    };

                    for server in node_model.servers.values() {
                        if matches!(server.state, ServerStateModel::Pending) {
                            info!("auto accepting server: {}", server.endpoint_id);
                            if let Err(e) = core.accept_connection(&server.endpoint_id) {
                                error!("error auto accepting server {}: {e:#}", server.endpoint_id);
                            }
                        }
                    }
                }
            });
        }

        let library_model = core.get_library_model()?;
        let node_model = core.get_node_model()?;
        let stats_model = core.get_stats_model()?;

        Ok(Self {
            running: true,
            events,

            core,

            mode: AppMode::default(),
            screen: AppScreen::default(),

            messages: Vec::new(),

            log_state: LogState::default(),
            command_state: TextState::default(),

            library_model,
            node_model,
            stats_model,

            transcode_format: Some(TranscodeFormat::Opus128),
        })
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => {
                    if let crossterm::event::Event::Key(key_event) = event {
                        self.handle_key_events(key_event)?
                    }
                }
                Event::App(app_event) => self
                    .handle_app_events(app_event)
                    .context("handling app event failed")?,
            }
        }

        // shut down core
        self.core.shutdown()?;

        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> anyhow::Result<()> {
        match self.mode {
            AppMode::Default => match (self.screen, key_event.code) {
                // : or / to enter command mode
                (_, KeyCode::Char(':') | KeyCode::Char('/')) => {
                    self.events.send(AppEvent::CommandMode)
                }

                // change screens
                (_, KeyCode::Char('1')) => self.events.send(AppEvent::Screen(AppScreen::Home)),
                (_, KeyCode::Char('2')) => self.events.send(AppEvent::Screen(AppScreen::Log)),
                (_, KeyCode::Char('3')) => self.events.send(AppEvent::Screen(AppScreen::Stats)),
                (_, KeyCode::Char('?')) => self.events.send(AppEvent::Screen(AppScreen::Help)),

                // esc or q to quit
                (_, KeyCode::Esc | KeyCode::Char('q')) => self.events.send(AppEvent::Exit),
                // ctrl+c to quit
                (_, KeyCode::Char('c' | 'C')) if key_event.modifiers == KeyModifiers::CONTROL => {
                    self.events.send(AppEvent::Exit)
                }

                // log screen
                (AppScreen::Log, KeyCode::Up) => {
                    self.log_state.scroll_up();
                }
                (AppScreen::Log, KeyCode::Down) => {
                    self.log_state.scroll_down();
                }
                (AppScreen::Log, KeyCode::PageUp) => {
                    self.log_state.scroll_page_up();
                }
                (AppScreen::Log, KeyCode::PageDown) => {
                    self.log_state.scroll_page_down();
                }
                (AppScreen::Log, KeyCode::Home | KeyCode::Char('g')) => {
                    self.log_state.scroll_to_top();
                }
                (AppScreen::Log, KeyCode::End | KeyCode::Char('G')) => {
                    self.log_state.scroll_to_bottom();
                }
                (AppScreen::Log, KeyCode::Char('f')) => {
                    self.log_state.toggle_tail();
                }

                _ => {}
            },

            AppMode::Command => {
                self.command_state.handle_key_event(key_event);

                match self.command_state.status() {
                    Status::Done => {
                        let command = self.command_state.value().to_string();

                        if let Err(e) = self.handle_command(command) {
                            error!("{e:#}");
                        }

                        self.events.send(AppEvent::ExitMode);
                    }
                    Status::Aborted => self.events.send(AppEvent::ExitMode),
                    Status::Pending => {}
                }
            }
        }

        Ok(())
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    pub fn handle_app_events(&mut self, app_event: AppEvent) -> anyhow::Result<()> {
        match app_event {
            AppEvent::Log(message) => self.messages.push(message),

            AppEvent::Exit => self.exit(),

            AppEvent::CommandMode => {
                self.mode = AppMode::Command;
                self.command_state.focus();
            }
            AppEvent::ExitMode => {
                self.mode = AppMode::Default;
                self.command_state = TextState::default();
            }

            AppEvent::Screen(screen) => {
                self.screen = screen;
            }

            AppEvent::LibraryModel(model) => {
                self.library_model = *model;
            }
            AppEvent::NodeModel(model) => {
                self.node_model = *model;
            }
            AppEvent::StatsModel(model) => {
                self.stats_model = *model;
            }
        }
        Ok(())
    }

    pub fn handle_command(&mut self, command: String) -> anyhow::Result<()> {
        let parts = command.split_whitespace().collect::<Vec<_>>();

        if parts.is_empty() {
            return Ok(());
        }

        match parts[0] {
            "q" | "quit" => self.events.send(AppEvent::Exit),

            "addlibrary" => {
                if parts.len() < 3 {
                    anyhow::bail!("usage: addlibrary <name> <path>");
                }

                let name = parts[1].to_string();
                let path = parts[2].to_string();
                self.core.add_library_root(name, path)?;
            }

            "removelibrary" => {
                if parts.len() < 2 {
                    anyhow::bail!("usage: removelibrary <name>");
                }

                let name = parts[1].to_string();
                self.core.remove_library_root(name)?;
            }

            "resetdb" => {
                self.core.reset_database()?;
                self.core.rescan_library()?;
            }

            "resetcaches" => {
                self.core.reset_caches()?;
            }

            "rescan" => {
                self.core.rescan_library()?;
            }

            "a" | "accept" => {
                info!("accepting pending servers");

                for server in self.node_model.servers.values() {
                    if matches!(server.state, ServerStateModel::Pending) {
                        info!("accepting server: {}", server.endpoint_id);
                        self.core.accept_connection(&server.endpoint_id)?;
                    }
                }
            }

            "t" | "trust" => {
                info!("accepting and trusting pending servers");

                for server in self.node_model.servers.values() {
                    if matches!(server.state, ServerStateModel::Pending) {
                        info!("accepting and trusting server: {}", server.endpoint_id);
                        self.core.accept_connection_and_trust(&server.endpoint_id)?;
                    }
                }
            }

            "c" | "connect" => {
                if parts.len() < 2 {
                    anyhow::bail!("usage: connect <endpoint_id>");
                }

                let endpoint_id = parts[1].to_string();

                info!("connecting to node: {}", endpoint_id);

                let core = self.core.clone();
                let transcode_format = self.transcode_format;
                tokio::spawn(async move {
                    if let Err(e) = core.connect(transcode_format, &endpoint_id).await {
                        error!("error connecting to node {}: {e:#}", endpoint_id);
                    }
                });
            }

            "dc" | "disconnect" => {
                info!("disconnecting everything");

                for client in self.node_model.clients.values() {
                    self.core.close_client(&client.endpoint_id)?;
                }

                for server in self.node_model.servers.values() {
                    self.core.close_server(&server.endpoint_id)?;
                }
            }

            "dl" | "download" => {
                if parts.len() < 2 {
                    anyhow::bail!("usage: download <client #>");
                }

                let client_num = parts[1]
                    .parse::<usize>()
                    .context("failed to parse client number")?;

                if client_num == 0 {
                    anyhow::bail!("client number must be greater than 0");
                }

                let client_model = self
                    .node_model
                    .clients
                    .values()
                    .filter(|c| matches!(c.state, ClientStateModel::Accepted))
                    .nth(client_num - 1)
                    .ok_or_else(|| anyhow::anyhow!("client number out of range"))?;

                let endpoint_id = client_model.endpoint_id.clone();
                let download_requests = client_model
                    .index
                    .as_ref()
                    .ok_or(anyhow::anyhow!("client index not available"))?
                    .iter()
                    .map(|item| DownloadRequestModel {
                        endpoint_id: endpoint_id.clone(),
                        root: item.root.clone(),
                        path: item.path.clone(),
                    })
                    .collect::<Vec<_>>();

                info!(
                    "downloading all {} items from client: {}",
                    download_requests.len(),
                    client_num
                );

                let core = self.core.clone();
                tokio::spawn(async move {
                    if let Err(e) = core.set_download_directory("/tmp/musicopy-dl") {
                        error!("error setting download directory: {e:#}");
                        return;
                    }

                    if let Err(e) = core.set_downloads(&endpoint_id, download_requests) {
                        error!("error downloading from client {}: {e:#}", client_num);
                    }
                });
            }

            "dlrand" => {
                if parts.len() < 2 {
                    anyhow::bail!("usage: dlrand <client #>");
                }

                let client_num = parts[1]
                    .parse::<usize>()
                    .context("failed to parse client number")?;

                if client_num == 0 {
                    anyhow::bail!("client number must be greater than 0");
                }

                let client_model = self
                    .node_model
                    .clients
                    .values()
                    .filter(|c| matches!(c.state, ClientStateModel::Accepted))
                    .nth(client_num - 1)
                    .ok_or_else(|| anyhow::anyhow!("client number out of range"))?;

                let fractions = 10;
                let random_fraction = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_secs() as usize
                    % fractions;

                let endpoint_id = client_model.endpoint_id.to_string();
                let download_requests = client_model
                    .index
                    .as_ref()
                    .ok_or(anyhow::anyhow!("client index not available"))?
                    .iter()
                    .enumerate()
                    .flat_map(|(i, item)| {
                        if i % fractions == random_fraction {
                            Some(DownloadRequestModel {
                                endpoint_id: endpoint_id.clone(),
                                root: item.root.clone(),
                                path: item.path.clone(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                info!(
                    "downloading {} random items from client: {}",
                    download_requests.len(),
                    client_num
                );

                let core = self.core.clone();
                tokio::spawn(async move {
                    if let Err(e) = core.set_download_directory("/tmp/musicopy-dl") {
                        error!("error setting download directory: {e:#}");
                        return;
                    }

                    if let Err(e) = core.set_downloads(&endpoint_id, download_requests) {
                        error!("error downloading from client {}: {e:#}", client_num);
                    }
                });
            }

            "p" | "pause" => {
                info!("pausing all downloads");

                for client in self.node_model.clients.values() {
                    if matches!(client.state, ClientStateModel::Accepted) {
                        self.core.pause_downloads(&client.endpoint_id)?;
                    }
                }
            }

            "delete-unused-transcodes" => {
                self.core.delete_unused_transcodes()?;
            }
            "delete-all-transcodes" => {
                self.core.delete_all_transcodes()?;
            }

            "f" | "format" => {
                if parts.len() < 2 {
                    anyhow::bail!("usage: format <opus128|opus64|mp3v0|mp3v5|none>");
                }

                let format = match parts[1] {
                    "opus128" => Some(TranscodeFormat::Opus128),
                    "opus64" => Some(TranscodeFormat::Opus64),
                    "mp3v0" => Some(TranscodeFormat::Mp3V0),
                    "mp3v5" => Some(TranscodeFormat::Mp3V5),
                    "none" => None,
                    _ => anyhow::bail!("unknown format: {}", parts[1]),
                };
                self.transcode_format = format;
            }

            "connectinfo" => {
                for server in self.node_model.servers.values() {
                    info!(
                        "server {}: status={:?} remote_addr={} latency_ms={:?}",
                        server.endpoint_id, server.state, server.connection_type, server.latency_ms,
                    );
                }

                for client in self.node_model.clients.values() {
                    info!(
                        "client {}: status={:?} remote_addr={} latency_ms={:?}",
                        client.endpoint_id, client.state, client.connection_type, client.latency_ms
                    );
                }
            }

            "exportlogs" => {
                let bytes = self.core.export_logs()?;
                if bytes.is_empty() {
                    error!("exportlogs: log data is empty");
                } else {
                    let timestamp = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)?
                        .as_secs();
                    let path = std::env::temp_dir().join(format!("musicopy-logs-{timestamp}.txt"));
                    std::fs::write(&path, &bytes).context("failed to write log export")?;
                    info!(
                        "exportlogs: wrote {} bytes to {}. note that this only includes logs from desktop runs. the tui does not collect logs",
                        bytes.len(),
                        path.display()
                    );
                }
            }

            "help" | "h" | "?" => {
                app_send!(AppEvent::Screen(AppScreen::Help));
            }

            _ => {
                anyhow::bail!("unknown command: {command}");
            }
        }
        Ok(())
    }

    /// Exit the app.
    fn exit(&mut self) {
        self.running = false;
    }
}

struct AppEventHandler;

impl musicopy::EventHandler for AppEventHandler {
    fn on_library_model_snapshot(&self, model: LibraryModel) {
        app_send!(AppEvent::LibraryModel(Box::new(model)));
    }

    fn on_node_model_snapshot(&self, model: NodeModel) {
        app_send!(AppEvent::NodeModel(Box::new(model)));
    }

    fn on_stats_model_snapshot(&self, model: StatsModel) {
        app_send!(AppEvent::StatsModel(Box::new(model)));
    }
}
