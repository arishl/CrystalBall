use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};

pub const TERMINAL_BG: egui::Color32 = egui::Color32::from_rgb(8, 8, 9);
pub const TERMINAL_PANEL_BG: egui::Color32 = egui::Color32::from_rgb(18, 18, 20);
pub const TERMINAL_TEXT: egui::Color32 = egui::Color32::from_rgb(238, 238, 239);
pub const TERMINAL_MUTED: egui::Color32 = egui::Color32::from_rgb(150, 150, 154);
pub const TERMINAL_ACCENT: egui::Color32 = egui::Color32::from_rgb(206, 162, 255);
pub const TERMINAL_ERROR: egui::Color32 = egui::Color32::from_rgb(255, 112, 126);
const TERMINAL_SUCCESS: egui::Color32 = egui::Color32::from_rgb(118, 230, 162);
const TERMINAL_WARNING: egui::Color32 = egui::Color32::from_rgb(255, 210, 104);
const TERMINAL_BLUE: egui::Color32 = egui::Color32::from_rgb(118, 176, 255);
const TERMINAL_VM: egui::Color32 = egui::Color32::from_rgb(255, 82, 96);

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
    let terminal_input_id = ui.make_persistent_id("terminal_input");
    let environment = terminal_environment(cwd);

    let terminal_response = egui::Frame::default()
        .fill(TERMINAL_PANEL_BG)
        .stroke(environment.stroke())
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
                if let Some(label) = environment.label() {
                    ui.label(
                        egui::RichText::new(label)
                            .monospace()
                            .color(environment.color()),
                    );
                }
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
                    let terminal_has_focus =
                        ui.memory(|memory| memory.has_focus(terminal_input_id));
                    let mut complete_requested = false;
                    let mut submit_requested = false;
                    let mut clear_requested = false;

                    if terminal_has_focus
                        && ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
                        })
                    {
                        complete_requested = true;
                    }

                    if terminal_has_focus
                        && ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                        })
                    {
                        submit_requested = true;
                    }

                    if terminal_has_focus
                        && (ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::CTRL, egui::Key::L)
                        }) || ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::COMMAND, egui::Key::K)
                        }))
                    {
                        clear_requested = true;
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
                                .hint_text("run a command")
                                .lock_focus(true),
                        );

                        if terminal.input.contains('\t') {
                            terminal.input.retain(|char| char != '\t');
                            complete_requested = true;
                        }

                        if complete_requested {
                            terminal.complete_input(cwd);
                            ui.memory_mut(|memory| memory.request_focus(terminal_input_id));
                        }

                        if clear_requested {
                            terminal.clear();
                            ui.memory_mut(|memory| memory.request_focus(terminal_input_id));
                        }

                        if response.changed() {
                            terminal.update_completion_preview(cwd);
                        }

                        if submit_requested {
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
                                let color = if item.ends_with('/') {
                                    TERMINAL_BLUE
                                } else {
                                    TERMINAL_MUTED
                                };
                                ui.label(egui::RichText::new(item).monospace().color(color));
                            }
                        });
                    }
                });
        })
        .response;

    if terminal_response.clicked() {
        ui.memory_mut(|memory| memory.request_focus(terminal_input_id));
    }

    command_to_run
}

#[derive(Clone, Copy)]
enum TerminalEnvironment {
    Normal,
    VirtualEnv,
    VirtualMachine,
}

impl TerminalEnvironment {
    fn color(self) -> egui::Color32 {
        match self {
            Self::Normal => egui::Color32::from_rgb(54, 54, 58),
            Self::VirtualEnv => TERMINAL_SUCCESS,
            Self::VirtualMachine => TERMINAL_VM,
        }
    }

    fn stroke(self) -> egui::Stroke {
        match self {
            Self::Normal => egui::Stroke::new(1.0, self.color()),
            Self::VirtualEnv | Self::VirtualMachine => egui::Stroke::new(2.0, self.color()),
        }
    }

    fn label(self) -> Option<&'static str> {
        match self {
            Self::Normal => None,
            Self::VirtualEnv => Some("[venv]"),
            Self::VirtualMachine => Some("[vm]"),
        }
    }
}

fn terminal_environment(cwd: &Path) -> TerminalEnvironment {
    if is_virtual_machine_environment() {
        TerminalEnvironment::VirtualMachine
    } else if is_virtualenv_environment(cwd) {
        TerminalEnvironment::VirtualEnv
    } else {
        TerminalEnvironment::Normal
    }
}

fn is_virtualenv_environment(cwd: &Path) -> bool {
    env::var_os("VIRTUAL_ENV").is_some()
        || env::var_os("CONDA_PREFIX").is_some()
        || find_project_venv(cwd)
}

fn find_project_venv(cwd: &Path) -> bool {
    for ancestor in cwd.ancestors() {
        for name in [".venv", "venv", "env"] {
            if ancestor.join(name).join("pyvenv.cfg").is_file() {
                return true;
            }
        }

        if ancestor.join("pyvenv.cfg").is_file() {
            return true;
        }
    }

    false
}

fn is_virtual_machine_environment() -> bool {
    static IS_VM: OnceLock<bool> = OnceLock::new();
    *IS_VM.get_or_init(|| {
        env::var_os("container").is_some()
            || env::var_os("CONTAINER").is_some()
            || env::var_os("DOCKER_CONTAINER").is_some()
            || env::var_os("WSL_DISTRO_NAME").is_some()
            || env::var_os("VAGRANT").is_some()
            || env::var_os("MULTIPASS_INSTANCE").is_some()
            || env::var_os("PARALLELS_VM").is_some()
            || Path::new("/.dockerenv").exists()
            || Path::new("/run/.containerenv").exists()
            || proc_cgroup_mentions_virtualization()
    })
}

fn proc_cgroup_mentions_virtualization() -> bool {
    let Ok(cgroup) = fs::read_to_string("/proc/1/cgroup") else {
        return false;
    };
    let cgroup = cgroup.to_ascii_lowercase();
    ["docker", "kubepods", "containerd", "lxc", "podman"]
        .iter()
        .any(|needle| cgroup.contains(needle))
}

fn show_terminal_line(ui: &mut egui::Ui, line: &TerminalLine) {
    match line {
        TerminalLine::Command { cwd, command } => {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} $", compact_path(cwd)))
                        .monospace()
                        .strong()
                        .color(TERMINAL_SUCCESS),
                );
                ui.label(
                    egui::RichText::new(command)
                        .monospace()
                        .color(TERMINAL_ACCENT),
                );
            });
        }
        TerminalLine::Output(output) => {
            ui.add(egui::Label::new(ansi_layout_job(output, TERMINAL_TEXT)).wrap());
        }
        TerminalLine::Error(error) => {
            ui.add(egui::Label::new(ansi_layout_job(error, TERMINAL_ERROR)).wrap());
        }
    }
}

fn ansi_layout_job(text: &str, default_color: egui::Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut color = default_color;
    let mut buffer = String::new();
    let mut chars = text.chars().peekable();

    while let Some(char) = chars.next() {
        if char == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            flush_ansi_buffer(&mut job, &mut buffer, color);

            let mut code = String::new();
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
                code.push(next);
            }
            color = ansi_color(&code, default_color).unwrap_or(color);
        } else {
            buffer.push(char);
        }
    }

    flush_ansi_buffer(&mut job, &mut buffer, color);
    job
}

fn flush_ansi_buffer(job: &mut LayoutJob, buffer: &mut String, color: egui::Color32) {
    if buffer.is_empty() {
        return;
    }

    job.append(
        buffer,
        0.0,
        TextFormat {
            font_id: egui::TextStyle::Monospace.resolve(&egui::Style::default()),
            color,
            ..Default::default()
        },
    );
    buffer.clear();
}

fn ansi_color(code: &str, default_color: egui::Color32) -> Option<egui::Color32> {
    let mut color = None;
    for part in code.split(';') {
        color = match part {
            "0" | "39" => Some(default_color),
            "30" | "90" => Some(TERMINAL_MUTED),
            "31" | "91" => Some(TERMINAL_ERROR),
            "32" | "92" => Some(TERMINAL_SUCCESS),
            "33" | "93" => Some(TERMINAL_WARNING),
            "34" | "94" => Some(TERMINAL_BLUE),
            "35" | "95" => Some(TERMINAL_ACCENT),
            "36" | "96" => Some(egui::Color32::from_rgb(108, 224, 224)),
            "37" | "97" => Some(TERMINAL_TEXT),
            _ => color,
        };
    }
    color
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
    if input.is_empty() {
        return None;
    }

    if input
        .chars()
        .last()
        .is_some_and(|char| char.is_ascii_whitespace())
    {
        return Some((input.len(), ""));
    }

    let token_start = input
        .char_indices()
        .rev()
        .find_map(|(index, char)| {
            char.is_ascii_whitespace()
                .then_some(index + char.len_utf8())
        })
        .unwrap_or(0);

    Some((token_start, &input[token_start..]))
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
        let mut completed = escape_completion_token(&token_path.to_string_lossy());
        completed.push(if entry.is_dir { '/' } else { ' ' });
        completed
    } else {
        let mut token_path = PathBuf::from(token);
        token_path.set_file_name(prefix);
        escape_completion_token(&token_path.to_string_lossy())
    };

    Some(format!("{}{}", &input[..token_start], completed_token))
}

fn escape_completion_token(token: &str) -> String {
    token
        .chars()
        .flat_map(|char| {
            if char.is_ascii_whitespace() {
                ['\\', char]
            } else {
                ['\0', char]
            }
        })
        .filter(|char| *char != '\0')
        .collect()
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
