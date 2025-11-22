//! The app TUI.

pub mod log;

use crate::app::{App, AppMode, AppScreen};
use musicopy::{
    library::transcode::TranscodePolicy,
    node::{ClientStateModel, ServerStateModel, TransferJobProgressModel},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Tabs, Widget},
};
use std::time::SystemTime;
use tui_widgets::prompts::{Prompt, TextPrompt};

impl<'a> App<'a> {
    // we're using this instead of Widget::render because we also need the
    // frame to use TextPrompt
    pub fn render(&mut self, frame: &mut Frame) {
        use Constraint::{Length, Min};

        let [header_area, inner_area] = {
            let show_command = self.mode == AppMode::Command;

            let mut constraints = vec![
                // header area
                Length(1),
                // inner area
                Min(0),
            ];
            if show_command {
                constraints.push(Length(3))
            }

            let vertical = Layout::vertical(constraints);
            let areas = vertical.split(frame.area());

            let header_area = areas[0];
            let inner_area = areas[1];

            let mut next_area = 1;
            if show_command {
                next_area += 1;
                let command_area = areas[next_area];

                self.render_command(frame, command_area);
            }

            [header_area, inner_area]
        };

        let horizontal = Layout::horizontal([Min(0), Length(10), Length(12)]);
        let [tabs_area, id_area, title_area] = horizontal.areas(header_area);

        // tabs
        let selected_tab_index = match self.screen {
            AppScreen::Home => 0,
            AppScreen::Log => 1,
            AppScreen::Help => 2,
        };
        let titles = ["Home", "Log", "Help"]
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
                let key = if s == "Help" {
                    "?".blue().bold()
                } else {
                    (i + 1).to_string().blue().bold()
                };

                if i == selected_tab_index {
                    Line::from(vec!["[".blue().bold(), key, "] ".blue().bold(), s.into()])
                } else {
                    Line::from(vec!["<".blue().bold(), key, "> ".blue().bold(), s.into()])
                }
            })
            .collect::<Vec<_>>();
        Tabs::new(titles)
            .select(None)
            .padding("", "")
            .divider(" ")
            .render(tabs_area, frame.buffer_mut());

        shorten_id(self.node_model.node_id.clone())
            .yellow()
            .render(id_area, frame.buffer_mut());

        // title
        "Musicopy TUI".bold().render(title_area, frame.buffer_mut());

        match self.screen {
            AppScreen::Home => {
                self.render_home_screen(frame, inner_area);
            }
            AppScreen::Log => {
                self.render_log_screen(frame, inner_area);
            }
            AppScreen::Help => {
                self.render_help_screen(frame, inner_area);
            }
        }
    }

    fn render_command(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered().border_set(border::THICK);

        TextPrompt::from("Command")
            .with_block(block)
            .draw(frame, area, &mut self.command_state);
    }

    fn render_home_screen(&mut self, frame: &mut Frame, area: Rect) {
        let title = Line::from(" Status ".bold());
        let instructions = Line::from(vec![
            " Command ".into(),
            "<:>".blue().bold(),
            " Quit ".into(),
            "<q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_top(instructions.right_aligned())
            .border_set(border::THICK);

        let trusted_nodes = self
            .node_model
            .trusted_nodes
            .iter()
            .map(shorten_id)
            .collect::<Vec<_>>()
            .join(", ");

        let recent_servers = self
            .node_model
            .recent_servers
            .iter()
            .map(|n| format!("{} ({})", shorten_id(&n.node_id), n.connected_at))
            .collect::<Vec<_>>()
            .join(", ");

        let active_servers = self
            .node_model
            .servers
            .values()
            .filter(|s| matches!(s.state, ServerStateModel::Accepted))
            .map(|s| format!("{} ({})", shorten_id(&s.node_id), s.name))
            .collect::<Vec<_>>()
            .join(", ");
        let pending_servers = self
            .node_model
            .servers
            .values()
            .filter(|s| matches!(s.state, ServerStateModel::Pending))
            .map(|s| format!("{} ({})", shorten_id(&s.node_id), s.name))
            .collect::<Vec<_>>()
            .join(", ");
        let closed_servers = self
            .node_model
            .servers
            .values()
            .filter_map(|s| match &s.state {
                ServerStateModel::Closed { error } => Some(format!(
                    "{} ({}) [{}]",
                    shorten_id(&s.node_id),
                    s.name,
                    error.as_deref().unwrap_or("no error")
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(", ");

        let active_clients = self
            .node_model
            .clients
            .values()
            .filter(|c| matches!(c.state, ClientStateModel::Accepted))
            .map(|c| format!("{} ({})", shorten_id(&c.node_id), c.name))
            .collect::<Vec<_>>()
            .join(", ");
        let pending_clients = self
            .node_model
            .clients
            .values()
            .filter(|c| matches!(c.state, ClientStateModel::Pending))
            .map(|c| format!("{} ({})", shorten_id(&c.node_id), c.name))
            .collect::<Vec<_>>()
            .join(", ");
        let closed_clients = self
            .node_model
            .clients
            .values()
            .filter_map(|c| match &c.state {
                ClientStateModel::Closed { error } => Some(format!(
                    "{} ({}) [{}]",
                    shorten_id(&c.node_id),
                    c.name,
                    error.as_deref().unwrap_or("no error")
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(", ");

        let mut lines = vec![
            Line::from("Network".bold()),
            Line::from(vec![
                "Node ID: ".into(),
                self.node_model.node_id.clone().yellow(),
            ]),
            Line::from(vec![
                "Home Relay: ".into(),
                self.node_model.home_relay.clone().yellow(),
            ]),
            Line::from(""),
            Line::from(vec!["Trusted Nodes: ".into(), trusted_nodes.yellow()]),
            Line::from(vec!["Recent Servers: ".into(), recent_servers.yellow()]),
            Line::from(""),
            Line::from(vec!["Pending Servers: ".into(), pending_servers.yellow()]),
            Line::from(vec!["Active Servers: ".into(), active_servers.yellow()]),
            Line::from(vec!["Closed Servers: ".into(), closed_servers.yellow()]),
            Line::from(""),
            Line::from(vec!["Pending Clients: ".into(), pending_clients.yellow()]),
            Line::from(vec!["Active Clients: ".into(), active_clients.yellow()]),
            Line::from(vec!["Closed Clients: ".into(), closed_clients.yellow()]),
            Line::from(""),
            Line::from("Library".bold()),
        ];

        if self.library_model.local_roots.is_empty() {
            // library help text if empty
            lines.push(Line::from(vec![
                "Empty, add a path using ".into(),
                ":addlibrary <name> <path>".blue(),
            ]));
        } else {
            // library roots
            lines.extend(self.library_model.local_roots.iter().map(|root| {
                Line::from(vec![
                    " - ".into(),
                    root.name.clone().blue(),
                    ": ".into(),
                    root.path.clone().blue(),
                    " (".green(),
                    root.num_files.to_string().green(),
                    ")".green(),
                ])
            }));

            let transcode_policy = match self.library_model.transcode_policy {
                TranscodePolicy::IfRequested => "IfRequested",
                TranscodePolicy::Always => "Always",
            };

            lines.extend(vec![
                Line::from(""),
                Line::from(vec![
                    "Transcodes: ".into(),
                    self.library_model
                        .transcode_count_queued
                        .get()
                        .to_string()
                        .green(),
                    " queued / ".into(),
                    self.library_model
                        .transcode_count_inprogress
                        .get()
                        .to_string()
                        .green(),
                    " in progress / ".into(),
                    self.library_model
                        .transcode_count_ready
                        .get()
                        .to_string()
                        .green(),
                    " ready / ".into(),
                    self.library_model
                        .transcode_count_failed
                        .get()
                        .to_string()
                        .green(),
                    " failed (policy: ".into(),
                    transcode_policy.green(),
                    ", size: ".into(),
                    format!("{:?}", self.library_model.transcodes_dir_size).green(),
                    ")".into(),
                ]),
            ])
        }

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // server jobs
        let server_jobs = self
            .node_model
            .servers
            .values()
            .flat_map(|server| {
                if server.transfer_jobs.is_empty() {
                    return vec![];
                }

                let mut count_transcoding = 0;
                let mut count_ready = 0;
                let mut count_inprogress = 0;
                let mut count_finished = 0;
                let mut count_failed = 0;

                for job in &server.transfer_jobs {
                    match &job.progress {
                        TransferJobProgressModel::Requested => {}
                        TransferJobProgressModel::Transcoding => count_transcoding += 1,
                        TransferJobProgressModel::Ready => count_ready += 1,
                        TransferJobProgressModel::InProgress { .. } => count_inprogress += 1,
                        TransferJobProgressModel::Finished { .. } => count_finished += 1,
                        TransferJobProgressModel::Failed { .. } => count_failed += 1,
                    }
                }

                let mut job_lines = vec![Line::from(vec![
                    " - ".into(),
                    shorten_id(&server.node_id).blue(),
                    ": ".into(),
                    count_transcoding.to_string().green(),
                    " transcoding / ".into(),
                    count_ready.to_string().green(),
                    " ready / ".into(),
                    count_inprogress.to_string().green(),
                    " in progress / ".into(),
                    count_finished.to_string().green(),
                    " finished / ".into(),
                    count_failed.to_string().green(),
                    " failed".into(),
                ])];

                // add lines for in-progress jobs
                for job in &server.transfer_jobs {
                    if let TransferJobProgressModel::InProgress { started_at, bytes } =
                        &job.progress
                    {
                        // calculate sizes in MB
                        let progress_mb = bytes.get() as f64 / 1_000_000.0;
                        let size_mb = job.file_size.unwrap_or(0) as f64 / 1_000_000.0;

                        // calculate percent
                        let progress_percent = if job.file_size.unwrap_or(0) > 0 {
                            (bytes.get() as f64 / job.file_size.unwrap_or(0) as f64) * 100.0
                        } else {
                            0.0
                        };

                        // calculate speed in MB/s
                        let elapsed = now - *started_at;
                        let bytes_per_second = if elapsed > 0 {
                            (bytes.get() as f64) / (elapsed as f64)
                        } else {
                            bytes.get() as f64
                        };
                        let mbytes_per_second = bytes_per_second / 1_000_000.0;

                        job_lines.push(Line::from(vec![
                            "   - ".into(),
                            job.file_root.clone().blue(),
                            "/".blue(),
                            job.file_path.clone().blue(),
                            " [".green(),
                            format!("{progress_mb:.1}").green(),
                            " MB/".green(),
                            format!("{size_mb:.1}").green(),
                            " MB ".green(),
                            format!("{progress_percent:.0}").green(),
                            "% ".green(),
                            format!("{mbytes_per_second:.2}").green(),
                            " MB/s".green(),
                            "]".green(),
                        ]));
                    }
                }

                job_lines
            })
            .collect::<Vec<_>>();
        if !server_jobs.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from("Outgoing Transfers".bold()));
            lines.extend(server_jobs);
        }

        // client jobs
        let client_jobs = self
            .node_model
            .clients
            .values()
            .flat_map(|client| {
                if client.transfer_jobs.is_empty() {
                    return vec![];
                }

                let mut count_requested = 0;
                let mut count_transcoding = 0;
                let mut count_ready = 0;
                let mut count_inprogress = 0;
                let mut count_finished = 0;
                let mut count_failed = 0;

                for job in &client.transfer_jobs {
                    match &job.progress {
                        TransferJobProgressModel::Requested => count_requested += 1,
                        TransferJobProgressModel::Transcoding => count_transcoding += 1,
                        TransferJobProgressModel::Ready => count_ready += 1,
                        TransferJobProgressModel::InProgress { .. } => count_inprogress += 1,
                        TransferJobProgressModel::Finished { .. } => count_finished += 1,
                        TransferJobProgressModel::Failed { .. } => count_failed += 1,
                    }
                }

                let mut job_lines = vec![Line::from(vec![
                    " - ".into(),
                    shorten_id(&client.node_id).blue(),
                    ": ".into(),
                    count_requested.to_string().green(),
                    " requested / ".into(),
                    count_transcoding.to_string().green(),
                    " transcoding / ".into(),
                    count_ready.to_string().green(),
                    " ready / ".into(),
                    count_inprogress.to_string().green(),
                    " in progress / ".into(),
                    count_finished.to_string().green(),
                    " finished / ".into(),
                    count_failed.to_string().green(),
                    " failed".into(),
                ])];

                // add lines for in-progress jobs
                for job in &client.transfer_jobs {
                    if let TransferJobProgressModel::InProgress { started_at, bytes } =
                        &job.progress
                    {
                        // calculate sizes in MB
                        let progress_mb = bytes.get() as f64 / 1_000_000.0;
                        let size_mb = job.file_size.unwrap_or(0) as f64 / 1_000_000.0;

                        // calculate percent
                        let progress_percent = if job.file_size.unwrap_or(0) > 0 {
                            (bytes.get() as f64 / job.file_size.unwrap_or(0) as f64) * 100.0
                        } else {
                            0.0
                        };

                        // calculate speed in MB/s
                        let elapsed = now - *started_at;
                        let bytes_per_second = if elapsed > 0 {
                            (bytes.get() as f64) / (elapsed as f64)
                        } else {
                            bytes.get() as f64
                        };
                        let mbytes_per_second = bytes_per_second / 1_000_000.0;

                        job_lines.push(Line::from(vec![
                            "   - ".into(),
                            job.file_root.clone().blue(),
                            "/".blue(),
                            job.file_path.clone().blue(),
                            " [".green(),
                            format!("{progress_mb:.1}").green(),
                            " MB/".green(),
                            format!("{size_mb:.1}").green(),
                            " MB ".green(),
                            format!("{progress_percent:.0}").green(),
                            "% ".green(),
                            format!("{mbytes_per_second:.2}").green(),
                            " MB/s".green(),
                            "]".green(),
                        ]));
                    }
                }

                job_lines
            })
            .collect::<Vec<_>>();
        if !client_jobs.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from("Incoming Transfers".bold()));
            lines.extend(client_jobs);
        }

        let status_text = Text::from(lines);

        Paragraph::new(status_text)
            .block(block)
            .render(area, frame.buffer_mut());
    }

    fn render_help_screen(&mut self, frame: &mut Frame, area: Rect) {
        let title = Line::from(" Help ".bold());
        let instructions = Line::from(vec![
            " Command ".into(),
            "<:>".blue().bold(),
            " Quit ".into(),
            "<q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_top(instructions.right_aligned())
            .border_set(border::THICK);

        let lines = vec![
            Line::from("Navigation".bold()),
            Line::from(vec![
                " - ".into(),
                "<1>".blue(),
                " and ".into(),
                "<2>".blue(),
                " to change screens.".into(),
            ]),
            Line::from(vec![
                " - ".into(),
                "<:>".blue(),
                " to open the command prompt.".into(),
            ]),
            Line::from(vec![
                " - ".into(),
                "<q>".blue(),
                " or ".into(),
                "<ctrl + c>".blue(),
                " to quit.".into(),
            ]),
        ];

        Paragraph::new(lines)
            .block(block)
            .render(area, frame.buffer_mut());
    }
}

fn shorten_id(node_id: impl Into<String>) -> String {
    let mut node_id = node_id.into();
    node_id.truncate(6);
    node_id.push_str("..");
    node_id
}
