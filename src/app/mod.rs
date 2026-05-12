use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use eframe::egui;

mod editor;
mod hover_preview;
mod memory;
mod preview;
mod shell;
mod theme;

use editor::{EditorAction, OpenTab, show_editor, show_tab_strip};
use hover_preview::PREVIEW_CLOSE_GRACE_SECONDS;
use memory::{MemoryAction, MemoryTrail};
use preview::HoverPreview;
use shell::{TerminalCommand, default_shell, parse_terminal_command};
use theme::{StatusMessage, apply_theme};

use crate::file_content::{TextDocument, read_preview, read_tab_document};
use crate::file_entry::FileEntry;
use crate::terminal::{TERMINAL_MUTED, TerminalState, show_terminal};

const DEFAULT_TERMINAL_HEIGHT: f32 = 220.0;
const MIN_TERMINAL_HEIGHT: f32 = 120.0;
const TERMINAL_DRAG_HANDLE_HEIGHT: f32 = 10.0;
pub struct FileExplorerApp {
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    selected: Option<PathBuf>,
    open_tabs: Vec<OpenTab>,
    active_tab: Option<usize>,
    pending_close_tab: Option<usize>,
    quick_open: QuickOpenState,
    folder_find: FolderFindState,
    memory_trail: MemoryTrail,
    terminal_collapsed: bool,
    hover_preview: Option<HoverPreview>,
    preview_row_hovered: bool,
    preview_keep_until: f64,
    preview_cache: HashMap<PathBuf, Result<Option<TextDocument>, String>>,
    terminal: TerminalState,
    terminal_height: f32,
    terminal_drag_start_height: Option<f32>,
    status: Option<StatusMessage>,
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
            pending_close_tab: None,
            quick_open: QuickOpenState::default(),
            folder_find: FolderFindState::default(),
            memory_trail: MemoryTrail::default(),
            terminal_collapsed: false,
            hover_preview: None,
            preview_row_hovered: false,
            preview_keep_until: 0.0,
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
                self.status = Some(StatusMessage::error(format!(
                    "Could not read {}: {err}",
                    self.current_dir.display()
                )));
            }
        }
    }

    fn open_directory(&mut self, path: PathBuf) {
        self.current_dir = path;
        self.selected = None;
        self.memory_trail.record_directory(&self.current_dir);
        self.refresh_entries();
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.selected = None;
            self.memory_trail.record_directory(&self.current_dir);
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
            self.memory_trail.record_file(&path);
            return;
        }

        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        let document = read_tab_document(&path).map_err(|err| err.to_string());
        let draft = document
            .as_ref()
            .ok()
            .and_then(Option::as_ref)
            .map(|document| document.text.clone())
            .unwrap_or_default();

        self.open_tabs.push(OpenTab {
            path: path.clone(),
            title,
            document,
            draft,
            dirty: false,
            find_query: String::new(),
        });
        self.active_tab = Some(self.open_tabs.len() - 1);
        self.memory_trail.record_file(&path);
    }

    fn request_close_tab(&mut self, index: usize) {
        if self.open_tabs.get(index).is_some_and(|tab| tab.dirty) {
            self.pending_close_tab = Some(index);
        } else {
            self.close_tab(index);
        }
    }

    fn close_tab(&mut self, index: usize) {
        self.open_tabs.remove(index);
        self.active_tab = match self.open_tabs.len() {
            0 => None,
            len if index >= len => Some(len - 1),
            _ => Some(index),
        };
    }

    fn save_tab(&mut self, index: usize) {
        let Some(tab) = self.open_tabs.get_mut(index) else {
            return;
        };

        match &mut tab.document {
            Ok(Some(document)) if document.truncated => {
                self.status = Some(StatusMessage::error(
                    "This file is too large to save from the preview editor.",
                ));
            }
            Ok(Some(document)) => match fs::write(&tab.path, &tab.draft) {
                Ok(()) => {
                    document.text.clone_from(&tab.draft);
                    tab.dirty = false;
                    self.preview_cache.remove(&tab.path);
                    self.memory_trail.record_save(&tab.path);
                    self.status = Some(StatusMessage::info(format!("Saved {}", tab.title)));
                }
                Err(err) => {
                    self.status = Some(StatusMessage::error(format!(
                        "Could not save {}: {err}",
                        tab.title
                    )));
                }
            },
            Ok(None) => {
                self.status = Some(StatusMessage::error("This does not look like a text file."));
            }
            Err(err) => {
                self.status = Some(StatusMessage::error(format!(
                    "Could not save {}: {err}",
                    tab.title
                )));
            }
        }
    }

    fn revert_tab(&mut self, index: usize) {
        let Some(tab) = self.open_tabs.get_mut(index) else {
            return;
        };

        tab.document = read_tab_document(&tab.path).map_err(|err| err.to_string());
        tab.draft = tab
            .document
            .as_ref()
            .ok()
            .and_then(Option::as_ref)
            .map(|document| document.text.clone())
            .unwrap_or_default();
        tab.dirty = false;
        self.preview_cache.remove(&tab.path);
        self.status = Some(StatusMessage::info(format!("Reverted {}", tab.title)));
    }

    fn show_file_row(&mut self, ui: &mut egui::Ui, entry: FileEntry) {
        let selected = self.selected.as_ref() == Some(&entry.path);
        let label = if entry.is_dir {
            format!("[dir]  {}", entry.name)
        } else {
            format!("[file] {}", entry.name)
        };
        let response = ui
            .selectable_label(selected, egui::RichText::new(label).monospace())
            .on_hover_cursor(if entry.is_dir {
                egui::CursorIcon::PointingHand
            } else {
                egui::CursorIcon::Text
            });

        if response.clicked() {
            self.selected = Some(entry.path.clone());

            if !entry.is_dir {
                self.open_file_tab(entry.path.clone());
            }
        }

        if response.double_clicked() {
            self.selected = Some(entry.path.clone());

            if entry.is_dir {
                self.open_directory(entry.path.clone());
            } else {
                self.open_file_tab(entry.path.clone());
            }
        }

        if !entry.is_dir && response.hovered() {
            self.preview_row_hovered = true;
            self.preview_keep_until = ui.input(|input| input.time) + PREVIEW_CLOSE_GRACE_SECONDS;
            self.hover_preview = Some(HoverPreview {
                path: entry.path.clone(),
                title: entry.name.clone(),
                anchor: response.rect.right_top() + egui::vec2(6.0, -4.0),
                source_rect: response.rect,
            });
        }
    }

    fn show_explorer(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let root = self.current_dir.clone();
            self.show_tree_directory(ui, &root, 0);
        });
    }

    fn show_tree_directory(&mut self, ui: &mut egui::Ui, path: &Path, depth: usize) {
        let Ok(read_dir) = fs::read_dir(path) else {
            return;
        };

        let mut entries: Vec<FileEntry> = read_dir
            .filter_map(Result::ok)
            .filter_map(|entry| FileEntry::from_path(entry.path()))
            .collect();
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        for entry in entries {
            if entry.is_dir {
                let header = egui::CollapsingHeader::new(
                    egui::RichText::new(format!("[dir]  {}", entry.name)).monospace(),
                )
                .id_salt(&entry.path)
                .default_open(false);

                let response = header.show(ui, |ui| {
                    self.show_tree_directory(ui, &entry.path, depth + 1);
                });

                if response.header_response.double_clicked() {
                    self.open_directory(entry.path.clone());
                }
            } else {
                self.show_file_row(ui, entry);
            }
        }
    }

    fn show_editor_panel(&mut self, ui: &mut egui::Ui) {
        if self.open_tabs.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Double-click a text file to edit it here.")
                        .color(TERMINAL_MUTED),
                );
            });
            return;
        }

        if let Some(index) = show_tab_strip(ui, &self.open_tabs, &mut self.active_tab) {
            self.request_close_tab(index);
        }

        ui.add_space(4.0);

        if let Some(index) = self
            .active_tab
            .filter(|index| *index < self.open_tabs.len())
        {
            let action = show_editor(ui, &mut self.open_tabs[index]);

            match action {
                EditorAction::None => {}
                EditorAction::Save => self.save_tab(index),
                EditorAction::Revert => self.revert_tab(index),
            }
        }
    }

    fn show_hover_preview(&mut self, ctx: &egui::Context) {
        let Some(preview) = self.hover_preview.clone() else {
            return;
        };

        let document = self.preview_for(&preview.path);
        hover_preview::show_hover_preview(
            ctx,
            &mut self.hover_preview,
            preview,
            self.preview_row_hovered,
            &mut self.preview_keep_until,
            document,
        );
    }

    fn run_terminal_command(&mut self, command: String) {
        self.terminal.push_command(&self.current_dir, &command);
        self.memory_trail.record_command(&command);

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
        if self.terminal_collapsed {
            egui::TopBottomPanel::bottom("terminal_collapsed")
                .exact_height(32.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Show Terminal").clicked() {
                            self.terminal_collapsed = false;
                        }
                        ui.label(egui::RichText::new("Terminal hidden").color(TERMINAL_MUTED));
                    });
                });
            return;
        }

        let max_height = (ctx.screen_rect().height() - 160.0).max(MIN_TERMINAL_HEIGHT);
        self.terminal_height = self.terminal_height.clamp(MIN_TERMINAL_HEIGHT, max_height);

        egui::TopBottomPanel::bottom("terminal")
            .exact_height(self.terminal_height)
            .show(ctx, |ui| {
                self.show_terminal_drag_handle(ui, max_height);
                if ui.button("Hide Terminal").clicked() {
                    self.terminal_collapsed = true;
                }

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
        apply_theme(ctx);
        self.preview_row_hovered = false;
        self.handle_global_shortcuts(ctx);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Up").clicked() {
                    self.go_up();
                }

                if ui.button("Refresh").clicked() {
                    self.refresh_entries();
                }

                if ui.button("Quick Open").clicked() {
                    self.quick_open.open = true;
                    self.quick_open.refresh(&self.current_dir);
                }

                if let Some(action) = self.memory_trail.show(ui) {
                    match action {
                        MemoryAction::OpenFile(path) => self.open_file_tab(path),
                        MemoryAction::OpenDirectory(path) => self.open_directory(path),
                    }
                }

                ui.separator();
                ui.label("Find");
                let find_response = ui.add(
                    egui::TextEdit::singleline(&mut self.folder_find.query)
                        .desired_width(160.0)
                        .hint_text("in folder"),
                );
                if find_response.changed() {
                    self.folder_find.search(&self.current_dir);
                }

                ui.separator();
                ui.label(
                    egui::RichText::new(self.current_dir.display().to_string())
                        .monospace()
                        .color(TERMINAL_MUTED),
                );
            });
        });

        self.show_terminal_panel(ctx);

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            if let Some(status) = &self.status {
                ui.colored_label(status.color(), &status.text);
            } else {
                ui.label("");
            }
        });

        egui::SidePanel::left("file_explorer")
            .resizable(true)
            .default_width(360.0)
            .show(ctx, |ui| {
                self.show_folder_find_results(ui);
                ui.add_space(6.0);
                self.show_explorer(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_editor_panel(ui);
        });

        self.show_hover_preview(ctx);
        self.show_quick_open(ctx);
        self.show_dirty_close_confirmation(ctx);
    }
}

impl FileExplorerApp {
    fn handle_global_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::P)) {
            self.quick_open.open = true;
            self.quick_open.refresh(&self.current_dir);
        }
    }

    fn show_quick_open(&mut self, ctx: &egui::Context) {
        if !self.quick_open.open {
            return;
        }

        egui::Window::new("Quick Open")
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.quick_open.query)
                        .desired_width(f32::INFINITY)
                        .hint_text("Type a file name"),
                );

                if response.changed() {
                    self.quick_open.update_matches();
                }

                if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                    self.quick_open.open = false;
                }

                ui.separator();
                let matches = self.quick_open.matches.clone();
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        for path in matches {
                            let label = path
                                .strip_prefix(&self.current_dir)
                                .unwrap_or(&path)
                                .display()
                                .to_string();
                            if ui.selectable_label(false, label).clicked() {
                                self.open_file_tab(path);
                                self.quick_open.open = false;
                            }
                        }
                    });
            });
    }

    fn show_dirty_close_confirmation(&mut self, ctx: &egui::Context) {
        let Some(index) = self.pending_close_tab else {
            return;
        };

        if index >= self.open_tabs.len() {
            self.pending_close_tab = None;
            return;
        }

        let title = self.open_tabs[index].title.clone();
        egui::Window::new("Unsaved changes")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("{title} has unsaved changes."));
                ui.horizontal(|ui| {
                    if ui.button("Save and Close").clicked() {
                        self.save_tab(index);
                        if index < self.open_tabs.len() && !self.open_tabs[index].dirty {
                            self.close_tab(index);
                        }
                        self.pending_close_tab = None;
                    }

                    if ui.button("Discard").clicked() {
                        self.close_tab(index);
                        self.pending_close_tab = None;
                    }

                    if ui.button("Cancel").clicked() {
                        self.pending_close_tab = None;
                    }
                });
            });
    }

    fn show_folder_find_results(&mut self, ui: &mut egui::Ui) {
        if self.folder_find.query.trim().is_empty() {
            return;
        }

        ui.label(
            egui::RichText::new(format!("{} folder matches", self.folder_find.results.len()))
                .color(TERMINAL_MUTED),
        );
        let results = self.folder_find.results.clone();
        egui::ScrollArea::vertical()
            .id_salt("folder_find_results")
            .max_height(120.0)
            .show(ui, |ui| {
                for path in results {
                    let label = path
                        .strip_prefix(&self.current_dir)
                        .unwrap_or(&path)
                        .display()
                        .to_string();
                    if ui.selectable_label(false, label).clicked() {
                        self.open_file_tab(path);
                    }
                }
            });
        ui.separator();
    }
}

#[derive(Default)]
struct QuickOpenState {
    open: bool,
    query: String,
    files: Vec<PathBuf>,
    matches: Vec<PathBuf>,
}

impl QuickOpenState {
    fn refresh(&mut self, root: &Path) {
        self.files.clear();
        collect_files(root, &mut self.files, 5000);
        self.update_matches();
    }

    fn update_matches(&mut self) {
        let query = self.query.to_lowercase();
        self.matches = self
            .files
            .iter()
            .filter(|path| {
                query.is_empty()
                    || path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| fuzzy_match(&query, &name.to_lowercase()))
            })
            .take(80)
            .cloned()
            .collect();
    }
}

#[derive(Default)]
struct FolderFindState {
    query: String,
    results: Vec<PathBuf>,
}

impl FolderFindState {
    fn search(&mut self, root: &Path) {
        self.results.clear();
        if self.query.trim().is_empty() {
            return;
        }

        let mut files = Vec::new();
        collect_files(root, &mut files, 3000);
        let needle = self.query.to_lowercase();
        self.results = files
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.to_lowercase().contains(&needle))
            })
            .take(80)
            .collect();
    }
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>, limit: usize) {
    if files.len() >= limit {
        return;
    }

    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.filter_map(Result::ok) {
        if files.len() >= limit {
            return;
        }

        let path = entry.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.') || name == "target")
        {
            continue;
        }

        if entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
            collect_files(&path, files, limit);
        } else {
            files.push(path);
        }
    }
}

fn fuzzy_match(query: &str, candidate: &str) -> bool {
    let mut chars = candidate.chars();
    query
        .chars()
        .all(|needle| chars.by_ref().any(|char| char == needle))
}
