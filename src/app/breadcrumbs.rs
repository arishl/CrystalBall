use std::path::{Path, PathBuf};

use eframe::egui;

use crate::terminal::TERMINAL_MUTED;

pub(crate) fn show(ui: &mut egui::Ui, current_dir: &Path) -> Option<PathBuf> {
    let mut destination = None;
    let crumbs = parts(current_dir);

    egui::ScrollArea::horizontal()
        .id_salt("current_dir_breadcrumbs")
        .max_height(ui.spacing().interact_size.y + 6.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 0.0);
                ui.spacing_mut().button_padding = egui::vec2(5.0, 2.0);
                ui.horizontal_centered(|ui| {
                    for (index, crumb) in crumbs.iter().enumerate() {
                        if index > 0 {
                            ui.label(egui::RichText::new("/").color(TERMINAL_MUTED).small());
                        }

                        let active = index + 1 == crumbs.len();
                        let text = egui::RichText::new(&crumb.label).monospace().small().color(
                            if active {
                                ui.visuals().text_color()
                            } else {
                                TERMINAL_MUTED
                            },
                        );
                        let response = if active {
                            ui.add_enabled(false, egui::Button::new(text).frame(false))
                        } else {
                            ui.add(egui::Button::new(text).frame(false))
                        }
                        .on_hover_text(crumb.path.display().to_string());

                        if response.clicked() && !active {
                            destination = Some(crumb.path.clone());
                        }
                    }
                });
            });
        });

    destination
}

fn parts(path: &Path) -> Vec<Crumb> {
    let mut parts = Vec::new();
    let mut current = PathBuf::new();

    for component in path.components() {
        current.push(component.as_os_str());
        parts.push(Crumb {
            label: component.as_os_str().to_string_lossy().into_owned(),
            path: current.clone(),
        });
    }

    if parts.is_empty() {
        parts.push(Crumb {
            label: path.display().to_string(),
            path: path.to_path_buf(),
        });
    }

    parts
}

struct Crumb {
    label: String,
    path: PathBuf,
}
