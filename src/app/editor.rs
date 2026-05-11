use std::path::PathBuf;

use eframe::egui;

use crate::file_content::{TextDocument, TextKind};
use crate::syntax_highlight::highlighted_code_job_with_width;
use crate::terminal::{TERMINAL_ACCENT, TERMINAL_MUTED, TERMINAL_PANEL_BG, TERMINAL_TEXT};

pub(crate) struct OpenTab {
    pub(crate) path: PathBuf,
    pub(crate) title: String,
    pub(crate) document: Result<Option<TextDocument>, String>,
    pub(crate) draft: String,
    pub(crate) dirty: bool,
}

pub(crate) enum EditorAction {
    None,
    Save,
    Revert,
}

pub(crate) fn show_tab_strip(
    ui: &mut egui::Ui,
    tabs: &[OpenTab],
    active_tab: &mut Option<usize>,
) -> Option<usize> {
    let mut tab_to_close = None;

    egui::ScrollArea::horizontal()
        .id_salt("editor_tab_strip")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (index, tab) in tabs.iter().enumerate() {
                    let response = show_tab_button(ui, tab, *active_tab == Some(index));

                    if response.selected {
                        *active_tab = Some(index);
                    }

                    if response.closed {
                        tab_to_close = Some(index);
                    }
                }
            });
        });

    tab_to_close
}

pub(crate) fn show_editor(ui: &mut egui::Ui, tab: &mut OpenTab) -> EditorAction {
    let mut action = EditorAction::None;

    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.heading(&tab.title);
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Revert").clicked() {
                action = EditorAction::Revert;
            }

            let can_save =
                matches!(&tab.document, Ok(Some(document)) if !document.truncated) && tab.dirty;
            if ui
                .add_enabled(can_save, egui::Button::new("Save"))
                .on_hover_text("Save changes to disk")
                .clicked()
            {
                action = EditorAction::Save;
            }

            if tab.dirty {
                ui.label(egui::RichText::new("unsaved").color(TERMINAL_ACCENT));
            }
        });
    });
    ui.add_space(8.0);

    match &tab.document {
        Ok(Some(document)) if document.truncated => show_read_only_document(ui, document),
        Ok(Some(document)) => {
            show_text_editor(ui, tab, document.kind);
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

    action
}

struct TabButtonResponse {
    selected: bool,
    closed: bool,
}

fn show_tab_button(ui: &mut egui::Ui, tab: &OpenTab, active: bool) -> TabButtonResponse {
    let title = if tab.dirty {
        format!("{} *", tab.title)
    } else {
        tab.title.clone()
    };
    let fill = if active {
        egui::Color32::from_rgb(35, 95, 75)
    } else {
        TERMINAL_PANEL_BG
    };

    let mut selected = false;
    let mut closed = false;

    egui::Frame::default()
        .fill(fill)
        .stroke(ui.visuals().widgets.inactive.bg_stroke)
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let title_response = ui
                    .selectable_label(active, egui::RichText::new(title).monospace())
                    .on_hover_text(format!("Edit {}", tab.title));
                selected |= title_response.clicked();

                let close_response = ui
                    .small_button("x")
                    .on_hover_text(format!("Close {}", tab.title));
                closed |= close_response.clicked();
            });
        });

    TabButtonResponse { selected, closed }
}

fn show_read_only_document(ui: &mut egui::Ui, document: &TextDocument) {
    ui.label(
        egui::RichText::new("Large file loaded read-only to avoid saving a partial copy.")
            .italics()
            .color(TERMINAL_MUTED),
    );
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .id_salt("read_only_large_file")
        .auto_shrink([false, false])
        .show(ui, |ui| match document.kind {
            TextKind::Code(language) => {
                let job =
                    highlighted_code_job_with_width(ui.available_width(), &document.text, language);
                ui.add(egui::Label::new(job).wrap());
            }
            TextKind::Markdown | TextKind::Plain => {
                ui.add(egui::Label::new(egui::RichText::new(&document.text).monospace()).wrap());
            }
        });
}

fn show_text_editor(ui: &mut egui::Ui, tab: &mut OpenTab, kind: TextKind) {
    let before = tab.draft.clone();
    let width = ui.available_width().max(240.0);
    let height = ui.available_height().max(360.0);
    let editor_size = egui::vec2(width, height);
    let text_width = (width - 24.0).max(120.0);
    let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
        let wrap_width = wrap_width.min(text_width).max(120.0);
        let job = match kind {
            TextKind::Code(language) => highlighted_code_job_with_width(wrap_width, text, language),
            TextKind::Markdown | TextKind::Plain => egui::text::LayoutJob::simple(
                text.to_owned(),
                egui::FontId::monospace(13.0),
                TERMINAL_TEXT,
                wrap_width,
            ),
        };
        ui.fonts(|fonts| fonts.layout_job(job))
    };

    egui::Frame::default()
        .fill(egui::Color32::from_rgb(18, 20, 24))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(42, 47, 55)))
        .corner_radius(6)
        .inner_margin(6)
        .show(ui, |ui| {
            let response = ui.add_sized(
                editor_size,
                egui::TextEdit::multiline(&mut tab.draft)
                    .id_salt(("file_editor", &tab.path))
                    .code_editor()
                    .desired_width(text_width)
                    .desired_rows(24)
                    .layouter(&mut layouter)
                    .hint_text("Empty file"),
            );

            if response.changed() || tab.draft != before {
                tab.dirty = true;
            }
        });
}
