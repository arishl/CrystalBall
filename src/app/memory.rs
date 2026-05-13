use std::path::{Path, PathBuf};

use eframe::egui;

use crate::terminal::{TERMINAL_BG, TERMINAL_MUTED};

const MAX_TRAIL_ITEMS: usize = 32;

#[derive(Default)]
pub(crate) struct MemoryTrail {
    items: Vec<MemoryItem>,
}

impl MemoryTrail {
    pub(crate) fn record_file(&mut self, path: &Path) {
        self.record(MemoryKind::File, path);
    }

    pub(crate) fn record_directory(&mut self, path: &Path) {
        self.record(MemoryKind::Directory, path);
    }

    pub(crate) fn record_save(&mut self, path: &Path) {
        self.record(MemoryKind::Save, path);
    }

    pub(crate) fn record_command(&mut self, command: &str) {
        let command = command.trim();
        if command.is_empty() {
            return;
        }

        self.push(MemoryItem {
            label: format!("$ {command}"),
            target: MemoryTarget::None,
        });
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<MemoryAction> {
        let mut action = None;

        ui.scope(|ui| {
            apply_memory_button_style(ui);
            egui::menu::menu_custom_button(
                ui,
                egui::Button::new(
                    egui::RichText::new(format!("Memory ({})", self.items.len())).size(13.0),
                )
                .frame(true),
                |ui| {
                    ui.set_min_width(320.0);

                    if self.items.is_empty() {
                        ui.label(egui::RichText::new("No memories yet.").color(TERMINAL_MUTED));
                        return;
                    }

                    if ui.button("Clear all").clicked() {
                        self.items.clear();
                        ui.close_menu();
                        return;
                    }

                    ui.separator();

                    let mut remove_index = None;
                    egui::ScrollArea::vertical()
                        .id_salt("project_memory_menu")
                        .max_height(320.0)
                        .show(ui, |ui| {
                            for index in (0..self.items.len()).rev() {
                                ui.horizontal(|ui| {
                                    let item = &self.items[index];
                                    let can_jump = !matches!(item.target, MemoryTarget::None);
                                    let response = ui.add_enabled(
                                        can_jump,
                                        egui::Button::new(&item.label)
                                            .min_size(egui::vec2(236.0, 0.0)),
                                    );

                                    if response.clicked() {
                                        action = match &item.target {
                                            MemoryTarget::File(path) => {
                                                Some(MemoryAction::OpenFile(path.clone()))
                                            }
                                            MemoryTarget::Directory(path) => {
                                                Some(MemoryAction::OpenDirectory(path.clone()))
                                            }
                                            MemoryTarget::None => None,
                                        };
                                        ui.close_menu();
                                    }

                                    if ui.small_button("x").on_hover_text("Remove").clicked() {
                                        remove_index = Some(index);
                                    }
                                });
                            }
                        });

                    if let Some(index) = remove_index {
                        self.items.remove(index);
                    }
                },
            );
        });

        action
    }

    fn record(&mut self, kind: MemoryKind, path: &Path) {
        let label = format!("{} {}", kind.prefix(), display_name(path));
        let target = match kind {
            MemoryKind::File | MemoryKind::Save => MemoryTarget::File(path.to_path_buf()),
            MemoryKind::Directory => MemoryTarget::Directory(path.to_path_buf()),
        };

        self.push(MemoryItem { label, target });
    }

    fn push(&mut self, item: MemoryItem) {
        if self
            .items
            .last()
            .is_some_and(|last| last.label == item.label && last.target == item.target)
        {
            return;
        }

        self.items.push(item);
        if self.items.len() > MAX_TRAIL_ITEMS {
            self.items.remove(0);
        }
    }
}

fn apply_memory_button_style(ui: &mut egui::Ui) {
    let visuals = &mut ui.style_mut().visuals;
    visuals.widgets.inactive.bg_fill = TERMINAL_BG;
    visuals.widgets.inactive.weak_bg_fill = TERMINAL_BG;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.open.bg_stroke = egui::Stroke::NONE;
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum MemoryAction {
    OpenFile(PathBuf),
    OpenDirectory(PathBuf),
}

#[derive(Clone, PartialEq, Eq)]
enum MemoryTarget {
    File(PathBuf),
    Directory(PathBuf),
    None,
}

struct MemoryItem {
    label: String,
    target: MemoryTarget,
}

enum MemoryKind {
    File,
    Directory,
    Save,
}

impl MemoryKind {
    fn prefix(&self) -> &'static str {
        match self {
            Self::File => "open",
            Self::Directory => "cd",
            Self::Save => "save",
        }
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}
