use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui;

pub const TERMINAL_BG: egui::Color32 = egui::Color32::from_rgb(8, 8, 9);
pub const TERMINAL_PANEL_BG: egui::Color32 = egui::Color32::from_rgb(18, 18, 20);
pub const TERMINAL_TEXT: egui::Color32 = egui::Color32::from_rgb(238, 238, 239);
pub const TERMINAL_MUTED: egui::Color32 = egui::Color32::from_rgb(150, 150, 154);
pub const TERMINAL_ACCENT: egui::Color32 = egui::Color32::from_rgb(245, 245, 246);
pub const TERMINAL_ERROR: egui::Color32 = egui::Color32::from_rgb(210, 210, 212);

#[derive(Default)]
pub struct TerminalState {
    input: String,
    completion_preview: Vec<String>,
    history: Vec<String>,
    history_index: Option<usize>,
    lines: Vec<TerminalLine>,
    should_scroll_to_bottom: bool,
}

impl TerminalState {
    pub fn push_command(&mut self, cwd: &Path, command: &str) {
        if self.history.last().is_none_or(|last| last != command) {
            self.history.push(command.to_owned());
        }
        self.history_index = None;
        self.lines.push(TerminalLine::Command {
            cwd: cwd.display().to_string(),
            command: command.to_owned(),
        });
        self.should_scroll_to_bottom = true;
    }

    pub fn push_output(&mut self, output: impl Into<String>) {
        self.lines.push(TerminalLine::Output(output.into()));
        self.should_scroll_to_bottom = true;
    }

    pub fn push_error(&mut self, error: impl Into<String>) {
        self.lines.push(TerminalLine::Error(error.into()));
        self.should_scroll_to_bottom = true;
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.should_scroll_to_bottom = true;
    }

    fn complete_input(&mut self, cwd: &Path) {
        let completion = complete_current_token(cwd, &self.input);
        self.completion_preview = completion.preview;

        if let Some(input) = completion.completed_input {
            self.input = input;
        }
    }

    fn update_completion_preview(&mut self, cwd: &Path) {
        self.completion_preview = complete_current_token(cwd, &self.input).preview;
    }

    fn history_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }

        let index = self
            .history_index
            .map(|index| index.saturating_sub(1))
            .unwrap_or(self.history.len() - 1);
        self.history_index = Some(index);
        self.input = self.history[index].clone();
    }

    fn history_next(&mut self) {
        let Some(index) = self.history_index else {
            return;
        };

        if index + 1 >= self.history.len() {
            self.history_index = None;
            self.input.clear();
        } else {
            let index = index + 1;
            self.history_index = Some(index);
            self.input = self.history[index].clone();
        }
    }
}

pub fn show_terminal(
    ui: &mut egui::Ui,
    terminal: &mut TerminalState,
    cwd: &Path,
) -> Option<String> {
    let mut command_to_run = None;
    let full_width = ui.available_width();

    egui::Frame::default()
        .fill(TERMINAL_PANEL_BG)
        .inner_margin(egui::Margin {
            left: 6,
            right: 6,
            top: 4,
            bottom: 6,
        })
        .show(ui, |ui| {
            ui.set_width(full_width);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Terminal")
                        .strong()
                        .color(TERMINAL_TEXT),
                );
                ui.label(
                    egui::RichText::new(cwd.display().to_string())
                        .monospace()
                        .color(TERMINAL_MUTED),
                );
            });

            ui.add_space(3.0);

            let completion_preview_height = if terminal.completion_preview.is_empty() {
                0.0
            } else {
                ui.spacing().interact_size.y * 2.0 + 12.0
            };
            let prompt_height = ui.spacing().interact_size.y + completion_preview_height + 24.0;
            let output_height = (ui.available_height() - prompt_height - 14.0).max(24.0);

            egui::Frame::default()
                .fill(TERMINAL_BG)
                .inner_margin(egui::Margin::same(5))
                .show(ui, |ui| {
                    ui.set_width(full_width - 12.0);

                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .max_width(ui.available_width())
                        .max_height(output_height)
                        .min_scrolled_height(output_height)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                for line in &terminal.lines {
                                    show_terminal_line(ui, line);
                                }

                                if terminal.should_scroll_to_bottom {
                                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                                    terminal.should_scroll_to_bottom = false;
                                }
                            });
                        });
                });

            ui.add_space(3.0);

            egui::Frame::default()
                .fill(TERMINAL_BG)
                .inner_margin(egui::Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.set_width(full_width - 12.0);
                    let terminal_input_id = ui.make_persistent_id("terminal_input");
                    let terminal_has_focus =
                        ui.memory(|memory| memory.has_focus(terminal_input_id));
                    let mut complete_requested = false;

                    if terminal_has_focus
                        && ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
                        })
                    {
                        complete_requested = true;
                    }

                    if terminal_has_focus
                        && ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
                        })
                    {
                        terminal.history_previous();
                        ui.memory_mut(|memory| memory.request_focus(terminal_input_id));
                    }

                    if terminal_has_focus
                        && ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                        })
                    {
                        terminal.history_next();
                        ui.memory_mut(|memory| memory.request_focus(terminal_input_id));
                    }

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("$")
                                .monospace()
                                .strong()
                                .color(TERMINAL_ACCENT),
                        );
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut terminal.input)
                                .id(terminal_input_id)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace)
                                .lock_focus(true),
                        );

                        if response.has_focus()
                            && ui.input_mut(|input| {
                                input.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
                            })
                        {
                            complete_requested = true;
                        }

                        if terminal.input.contains('\t') {
                            terminal.input.retain(|char| char != '\t');
                            complete_requested = true;
                        }

                        if complete_requested {
                            terminal.complete_input(cwd);
                            ui.memory_mut(|memory| memory.request_focus(terminal_input_id));
                        }

                        if response.changed() {
                            terminal.update_completion_preview(cwd);
                        }

                        let enter_pressed = ui.input(|input| input.key_pressed(egui::Key::Enter));
                        if response.lost_focus() && enter_pressed {
                            let command = terminal.input.trim().to_owned();
                            terminal.input.clear();
                            terminal.completion_preview.clear();

                            if !command.is_empty() {
                                command_to_run = Some(command);
                            }

                            response.request_focus();
                        }
                    });

                    if !terminal.completion_preview.is_empty() {
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_height(completion_preview_height);
                            for item in terminal.completion_preview.iter().take(8) {
                                ui.label(
                                    egui::RichText::new(item).monospace().color(TERMINAL_MUTED),
                                );
                            }
                        });
                    }
                });
        });

    command_to_run
}

fn show_terminal_line(ui: &mut egui::Ui, line: &TerminalLine) {
    match line {
        TerminalLine::Command { cwd, command } => {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} $", compact_path(cwd)))
                        .monospace()
                        .strong()
                        .color(TERMINAL_ACCENT),
                );
                ui.label(
                    egui::RichText::new(command)
                        .monospace()
                        .color(TERMINAL_TEXT),
                );
            });
        }
        TerminalLine::Output(output) => {
            ui.add(
                egui::Label::new(egui::RichText::new(output).monospace().color(TERMINAL_TEXT))
                    .wrap(),
            );
        }
        TerminalLine::Error(error) => {
            ui.add(
                egui::Label::new(egui::RichText::new(error).monospace().color(TERMINAL_ERROR))
                    .wrap(),
            );
        }
    }
}

fn compact_path(path: &str) -> String {
    if let Some(home) = std::env::var_os("HOME").and_then(|home| home.into_string().ok()) {
        if path == home {
            return "~".to_owned();
        }

        if let Some(rest) = path.strip_prefix(&(home + "/")) {
            return format!("~/{rest}");
        }
    }

    path.to_owned()
}

enum TerminalLine {
    Command { cwd: String, command: String },
    Output(String),
    Error(String),
}

struct CompletionResult {
    completed_input: Option<String>,
    preview: Vec<String>,
}

fn complete_current_token(cwd: &Path, input: &str) -> CompletionResult {
    let Some((token_start, token)) = current_token(input) else {
        return CompletionResult {
            completed_input: None,
            preview: Vec::new(),
        };
    };

    let (search_dir, file_prefix) = split_completion_token(cwd, token);
    let Ok(entries) = fs::read_dir(&search_dir) else {
        return CompletionResult {
            completed_input: None,
            preview: Vec::new(),
        };
    };

    let mut matches: Vec<CompletionMatch> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();

            if !name.starts_with(&file_prefix) {
                return None;
            }

            let is_dir = entry.file_type().ok()?.is_dir();
            Some(CompletionMatch { name, is_dir })
        })
        .collect();

    matches.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let preview = matches
        .iter()
        .take(8)
        .map(|entry| {
            if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            }
        })
        .collect();

    let completed_input = completion_text(input, token_start, token, &matches);

    CompletionResult {
        completed_input,
        preview,
    }
}

fn current_token(input: &str) -> Option<(usize, &str)> {
    let trimmed_end = input.trim_end();
    if trimmed_end.is_empty() {
        return None;
    }

    let token_start = trimmed_end
        .char_indices()
        .rev()
        .find_map(|(index, char)| {
            char.is_ascii_whitespace()
                .then_some(index + char.len_utf8())
        })
        .unwrap_or(0);

    Some((token_start, &trimmed_end[token_start..]))
}

fn split_completion_token(cwd: &Path, token: &str) -> (PathBuf, String) {
    let token_path = PathBuf::from(token);
    let parent = token_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let search_dir = match parent {
        Some(parent) if parent.is_absolute() => parent.to_path_buf(),
        Some(parent) => cwd.join(parent),
        None => cwd.to_path_buf(),
    };
    let file_prefix = token_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_owned();

    (search_dir, file_prefix)
}

fn completion_text(
    input: &str,
    token_start: usize,
    token: &str,
    matches: &[CompletionMatch],
) -> Option<String> {
    if matches.is_empty() {
        return None;
    }

    let prefix = common_prefix(matches.iter().map(|entry| entry.name.as_str()));
    if prefix.is_empty() {
        return None;
    }

    let completed_token = if matches.len() == 1 {
        let entry = &matches[0];
        let mut token_path = PathBuf::from(token);
        token_path.set_file_name(&entry.name);
        let mut completed = token_path.to_string_lossy().into_owned();
        completed.push(if entry.is_dir { '/' } else { ' ' });
        completed
    } else {
        let mut token_path = PathBuf::from(token);
        token_path.set_file_name(prefix);
        token_path.to_string_lossy().into_owned()
    };

    Some(format!("{}{}", &input[..token_start], completed_token))
}

fn common_prefix<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let mut values = values;
    let Some(first) = values.next() else {
        return String::new();
    };
    let mut prefix = first.to_owned();

    for value in values {
        while !value.starts_with(&prefix) {
            prefix.pop();
            if prefix.is_empty() {
                return prefix;
            }
        }
    }

    prefix
}

struct CompletionMatch {
    name: String,
    is_dir: bool,
}
