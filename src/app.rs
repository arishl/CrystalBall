use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use eframe::egui;

use crate::file_content::{TextDocument, TextKind, read_preview, read_tab_document};
use crate::file_entry::FileEntry;
use crate::markdown_view::show_markdown;
use crate::syntax_highlight::show_code;
use crate::terminal::{
    TERMINAL_ACCENT, TERMINAL_BG, TERMINAL_ERROR, TERMINAL_MUTED, TERMINAL_PANEL_BG, TERMINAL_TEXT,
    TerminalState, show_terminal,
};

const DEFAULT_TERMINAL_HEIGHT: f32 = 220.0;
const MIN_TERMINAL_HEIGHT: f32 = 120.0;
const TERMINAL_DRAG_HANDLE_HEIGHT: f32 = 10.0;

pub struct FileExplorerApp {
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    selected: Option<PathBuf>,
    open_tabs: Vec<OpenTab>,
    active_tab: Option<usize>,
    preview_cache: HashMap<PathBuf, Result<Option<TextDocument>, String>>,
    terminal: TerminalState,
    terminal_height: f32,
    terminal_drag_start_height: Option<f32>,
    status: Option<String>,
}

impl Default for FileExplorerApp {
    fn default() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut app = Self {
            current_dir,
            entries: Vec::new(),
            selected: None,
            open_tabs: Vec::new(),
            active_tab: None,
            preview_cache: HashMap::new(),
            terminal: TerminalState::default(),
            terminal_height: DEFAULT_TERMINAL_HEIGHT,
            terminal_drag_start_height: None,
            status: None,
        };
        app.refresh_entries();
        app
    }
}

impl FileExplorerApp {
    fn refresh_entries(&mut self) {
        self.entries.clear();
        self.preview_cache.clear();
        self.status = None;

        match fs::read_dir(&self.current_dir) {
            Ok(entries) => {
                self.entries = entries
                    .filter_map(Result::ok)
                    .filter_map(|entry| FileEntry::from_path(entry.path()))
                    .collect();

                self.entries.sort_by(|a, b| {
                    b.is_dir
                        .cmp(&a.is_dir)
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
            }
            Err(err) => {
                self.status = Some(format!(
                    "Could not read {}: {err}",
                    self.current_dir.display()
                ));
            }
        }
    }

    fn open_directory(&mut self, path: PathBuf) {
        self.current_dir = path;
        self.selected = None;
        self.refresh_entries();
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.selected = None;
            self.refresh_entries();
        }
    }

    fn preview_for(&mut self, path: &Path) -> Result<Option<TextDocument>, String> {
        self.preview_cache
            .entry(path.to_path_buf())
            .or_insert_with(|| read_preview(path).map_err(|err| err.to_string()))
            .clone()
    }

    fn open_file_tab(&mut self, path: PathBuf) {
        if let Some(index) = self.open_tabs.iter().position(|tab| tab.path == path) {
            self.active_tab = Some(index);
            return;
        }

        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        let document = read_tab_document(&path).map_err(|err| err.to_string());
        self.open_tabs.push(OpenTab {
            path,
            title,
            document,
        });
        self.active_tab = Some(self.open_tabs.len() - 1);
    }

    fn close_tab(&mut self, index: usize) {
        self.open_tabs.remove(index);
        self.active_tab = match self.open_tabs.len() {
            0 => None,
            len if index >= len => Some(len - 1),
            _ => Some(index),
        };
    }

    fn show_file_row(&mut self, ui: &mut egui::Ui, entry: FileEntry) {
        let selected = self.selected.as_ref() == Some(&entry.path);
        let icon = if entry.is_dir { "[D]" } else { "[F]" };
        let label = format!("{icon} {}", entry.name);
        let response = ui.selectable_label(selected, label);

        if response.clicked() {
            self.selected = Some(entry.path.clone());

            if !entry.is_dir {
                self.open_file_tab(entry.path.clone());
            }
        }

        if response.double_clicked() && entry.is_dir {
            self.open_directory(entry.path.clone());
        }

        if !entry.is_dir && response.hovered() {
            let preview = self.preview_for(&entry.path);

            response.on_hover_ui(|ui| {
                ui.set_max_width(540.0);
                ui.label(egui::RichText::new(&entry.name).strong());
                ui.separator();
                show_document_preview(ui, preview);
            });
        }
    }

    fn show_explorer(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for entry in self.entries.clone() {
                self.show_file_row(ui, entry);
            }
        });
    }

    fn show_tabs(&mut self, ui: &mut egui::Ui) {
        if self.open_tabs.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("Click a text or Markdown file to open it here.");
            });
            return;
        }

        let mut tab_to_close = None;

        ui.horizontal_wrapped(|ui| {
            for (index, tab) in self.open_tabs.iter().enumerate() {
                let active = self.active_tab == Some(index);

                if ui.selectable_label(active, &tab.title).clicked() {
                    self.active_tab = Some(index);
                }

                if ui.small_button("x").clicked() {
                    tab_to_close = Some(index);
                }
            }
        });

        if let Some(index) = tab_to_close {
            self.close_tab(index);
        }

        ui.separator();

        if let Some(tab) = self.active_tab.and_then(|index| self.open_tabs.get(index)) {
            egui::ScrollArea::vertical().show(ui, |ui| {
                show_open_tab(ui, tab);
            });
        }
    }

    fn run_terminal_command(&mut self, command: String) {
        self.terminal.push_command(&self.current_dir, &command);

        match parse_terminal_command(&command) {
            TerminalCommand::Clear => {
                self.terminal.clear();
            }
            TerminalCommand::ChangeDirectory(path) => {
                let next_dir = if path.is_absolute() {
                    path
                } else {
                    self.current_dir.join(path)
                };

                match fs::canonicalize(&next_dir) {
                    Ok(next_dir) if next_dir.is_dir() => {
                        self.current_dir = next_dir;
                        self.selected = None;
                        self.refresh_entries();
                    }
                    Ok(next_dir) => {
                        self.terminal
                            .push_error(format!("cd: not a directory: {}", next_dir.display()));
                    }
                    Err(err) => {
                        self.terminal.push_error(format!("cd: {err}"));
                    }
                }
            }
            TerminalCommand::Shell(command) => {
                let output = Command::new(default_shell())
                    .arg("-lc")
                    .arg(&command)
                    .current_dir(&self.current_dir)
                    .output();

                match output {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);

                        if !stdout.is_empty() {
                            self.terminal.push_output(stdout.trim_end().to_owned());
                        }

                        if !stderr.is_empty() {
                            self.terminal.push_error(stderr.trim_end().to_owned());
                        }

                        if stdout.is_empty() && stderr.is_empty() && !output.status.success() {
                            self.terminal
                                .push_error(format!("Command exited with {}", output.status));
                        }

                        self.refresh_entries();
                    }
                    Err(err) => {
                        self.terminal
                            .push_error(format!("Could not run command: {err}"));
                    }
                }
            }
        }
    }

    fn show_terminal_panel(&mut self, ctx: &egui::Context) {
        let max_height = (ctx.screen_rect().height() - 160.0).max(MIN_TERMINAL_HEIGHT);
        self.terminal_height = self.terminal_height.clamp(MIN_TERMINAL_HEIGHT, max_height);

        egui::TopBottomPanel::bottom("terminal")
            .exact_height(self.terminal_height)
            .show(ctx, |ui| {
                self.show_terminal_drag_handle(ui, max_height);

                if let Some(command) = show_terminal(ui, &mut self.terminal, &self.current_dir) {
                    self.run_terminal_command(command);
                }
            });
    }

    fn show_terminal_drag_handle(&mut self, ui: &mut egui::Ui, max_height: f32) {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), TERMINAL_DRAG_HANDLE_HEIGHT),
            egui::Sense::drag(),
        );
        let response = response.on_hover_cursor(egui::CursorIcon::ResizeVertical);

        let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        let y = rect.center().y;
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + 12.0, y),
                egui::pos2(rect.right() - 12.0, y),
            ],
            stroke,
        );

        if response.drag_started() {
            self.terminal_drag_start_height = Some(self.terminal_height);
        }

        if response.dragged() {
            let start_height = self
                .terminal_drag_start_height
                .unwrap_or(self.terminal_height);
            self.terminal_height =
                (start_height - response.drag_delta().y).clamp(MIN_TERMINAL_HEIGHT, max_height);
        }

        if response.drag_stopped() {
            self.terminal_drag_start_height = None;
        }
    }
}

impl eframe::App for FileExplorerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_terminal_theme(ctx);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Up").clicked() {
                    self.go_up();
                }

                if ui.button("Refresh").clicked() {
                    self.refresh_entries();
                }

                ui.separator();
                ui.label(self.current_dir.display().to_string());
            });
        });

        self.show_terminal_panel(ctx);

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            if let Some(status) = &self.status {
                ui.colored_label(egui::Color32::from_rgb(190, 70, 70), status);
            } else if let Some(selected) = &self.selected {
                ui.label(selected.display().to_string());
            } else {
                ui.label(
                    "Hover over text files to preview them. Markdown previews render as Markdown.",
                );
            }
        });

        egui::SidePanel::left("file_explorer")
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.heading("Files");
                ui.add_space(6.0);
                self.show_explorer(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_tabs(ui);
        });
    }
}

fn apply_terminal_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    let subtle_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(42, 47, 55));
    let active_stroke = egui::Stroke::new(1.0, TERMINAL_ACCENT);

    visuals.override_text_color = Some(TERMINAL_TEXT);
    visuals.panel_fill = TERMINAL_BG;
    visuals.window_fill = TERMINAL_PANEL_BG;
    visuals.extreme_bg_color = TERMINAL_BG;
    visuals.faint_bg_color = egui::Color32::from_rgb(28, 32, 38);
    visuals.code_bg_color = TERMINAL_BG;
    visuals.hyperlink_color = TERMINAL_ACCENT;
    visuals.error_fg_color = TERMINAL_ERROR;
    visuals.selection.bg_fill = egui::Color32::from_rgb(35, 95, 75);
    visuals.selection.stroke = egui::Stroke::new(1.0, TERMINAL_TEXT);
    visuals.window_stroke = subtle_stroke;
    visuals.widgets.noninteractive.bg_fill = TERMINAL_PANEL_BG;
    visuals.widgets.noninteractive.weak_bg_fill = TERMINAL_PANEL_BG;
    visuals.widgets.noninteractive.bg_stroke = subtle_stroke;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TERMINAL_TEXT);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(30, 34, 40);
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(30, 34, 40);
    visuals.widgets.inactive.bg_stroke = subtle_stroke;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TERMINAL_TEXT);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(38, 44, 51);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(38, 44, 51);
    visuals.widgets.hovered.bg_stroke = active_stroke;
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TERMINAL_TEXT);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(35, 95, 75);
    visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(35, 95, 75);
    visuals.widgets.active.bg_stroke = active_stroke;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, TERMINAL_TEXT);
    visuals.widgets.open.bg_fill = egui::Color32::from_rgb(38, 44, 51);
    visuals.widgets.open.weak_bg_fill = egui::Color32::from_rgb(38, 44, 51);
    visuals.widgets.open.bg_stroke = active_stroke;
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, TERMINAL_TEXT);
    visuals.warn_fg_color = TERMINAL_MUTED;
    visuals.button_frame = true;

    ctx.set_visuals(visuals);
}

enum TerminalCommand {
    Clear,
    ChangeDirectory(PathBuf),
    Shell(String),
}

fn parse_terminal_command(command: &str) -> TerminalCommand {
    let command = command.trim();

    if command == "clear" {
        return TerminalCommand::Clear;
    }

    if command == "cd" {
        return TerminalCommand::ChangeDirectory(home_dir());
    }

    if let Some(path) = command.strip_prefix("cd ") {
        return TerminalCommand::ChangeDirectory(expand_home(path.trim()));
    }

    TerminalCommand::Shell(command.to_owned())
}

fn expand_home(path: &str) -> PathBuf {
    if path == "~" {
        return home_dir();
    }

    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir().join(rest);
    }

    PathBuf::from(path)
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned())
}

struct OpenTab {
    path: PathBuf,
    title: String,
    document: Result<Option<TextDocument>, String>,
}

fn show_document_preview(ui: &mut egui::Ui, document: Result<Option<TextDocument>, String>) {
    match document {
        Ok(Some(document)) if document.text.is_empty() => {
            ui.label(egui::RichText::new("This file is empty.").italics());
        }
        Ok(Some(document)) => {
            show_document(ui, &document);
        }
        Ok(None) => {
            ui.label(egui::RichText::new("No text preview available.").italics());
        }
        Err(err) => {
            ui.colored_label(
                egui::Color32::from_rgb(190, 70, 70),
                format!("Could not preview file: {err}"),
            );
        }
    }
}

fn show_open_tab(ui: &mut egui::Ui, tab: &OpenTab) {
    ui.label(egui::RichText::new(tab.path.display().to_string()).weak());
    ui.add_space(8.0);

    match &tab.document {
        Ok(Some(document)) if document.text.is_empty() => {
            ui.label(egui::RichText::new("This file is empty.").italics());
        }
        Ok(Some(document)) => {
            show_document(ui, document);

            if document.truncated {
                ui.add_space(8.0);
                ui.label(egui::RichText::new("File truncated for display.").italics());
            }
        }
        Ok(None) => {
            ui.label(egui::RichText::new("This does not look like a text file.").italics());
        }
        Err(err) => {
            ui.colored_label(
                egui::Color32::from_rgb(190, 70, 70),
                format!("Could not open file: {err}"),
            );
        }
    }
}

fn show_document(ui: &mut egui::Ui, document: &TextDocument) {
    match document.kind {
        TextKind::Code(language) => show_code(ui, &document.text, language),
        TextKind::Markdown => show_markdown(ui, &document.text),
        TextKind::Plain => {
            ui.add(egui::Label::new(egui::RichText::new(&document.text).monospace()).wrap());
        }
    }
}
