use std::path::PathBuf;

use eframe::egui;

use crate::file_content::{TextDocument, TextKind};
use crate::syntax_highlight::highlighted_code_job_with_width;
use crate::terminal::{TERMINAL_ACCENT, TERMINAL_MUTED, TERMINAL_PANEL_BG, TERMINAL_TEXT};

const EDITOR_FONT_SIZE: f32 = 13.0;
const EDITOR_TEXT_MARGIN_X: f32 = 4.0;
const EDITOR_TEXT_MARGIN_Y: f32 = 2.0;

pub(crate) struct OpenTab {
    pub(crate) path: PathBuf,
    pub(crate) title: String,
    pub(crate) document: Result<Option<TextDocument>, String>,
    pub(crate) draft: String,
    pub(crate) dirty: bool,
    pub(crate) find_query: String,
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
                ui.spacing_mut().item_spacing.x = 4.0;
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

    let document_state = match &tab.document {
        Ok(Some(document)) => Ok(Some((document.kind, document.truncated))),
        Ok(None) => Ok(None),
        Err(err) => Err(format!("Could not open file: {err}")),
    };

    match document_state {
        Ok(Some((_kind, true))) => {
            if let Ok(Some(document)) = &tab.document {
                show_read_only_document(ui, document);
            }
        }
        Ok(Some((kind, false))) => {
            show_edit_document(ui, tab, kind);
        }
        Ok(None) => {
            ui.label(egui::RichText::new("This does not look like a text file.").italics());
        }
        Err(message) => {
            ui.colored_label(egui::Color32::from_rgb(210, 210, 212), message);
        }
    }

    ui.add_space(3.0);
    show_editor_controls(ui, tab, &mut action);

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
        egui::Color32::from_rgb(76, 76, 82)
    } else {
        TERMINAL_PANEL_BG
    };

    let mut selected = false;
    let mut closed = false;

    egui::Frame::default()
        .fill(fill)
        .stroke(ui.visuals().widgets.inactive.bg_stroke)
        .corner_radius(3)
        .inner_margin(egui::Margin::symmetric(4, 1))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let title_response = ui
                    .selectable_label(active, egui::RichText::new(title).monospace())
                    .on_hover_text(format!("Edit {}", tab.title));
                selected |= title_response.clicked();

                let close_response = ui
                    .add(
                        egui::Button::new(egui::RichText::new("x").size(9.0))
                            .fill(TERMINAL_PANEL_BG)
                            .min_size(egui::vec2(12.0, 12.0)),
                    )
                    .on_hover_text(format!("Close {}", tab.title));
                closed |= close_response.clicked();
            });
        });

    TabButtonResponse { selected, closed }
}

fn show_editor_controls(ui: &mut egui::Ui, tab: &mut OpenTab, action: &mut EditorAction) {
    egui::Frame::default()
        .fill(egui::Color32::from_rgb(12, 12, 14))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(42, 42, 46)))
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let can_save =
                    matches!(&tab.document, Ok(Some(document)) if !document.truncated) && tab.dirty;

                if ui
                    .add_enabled(can_save, compact_button("Save"))
                    .on_hover_text("Save changes to disk")
                    .clicked()
                {
                    *action = EditorAction::Save;
                }

                if ui.add(compact_button("Revert")).clicked() {
                    *action = EditorAction::Revert;
                }

                if tab.dirty {
                    ui.label(egui::RichText::new("unsaved").color(TERMINAL_ACCENT));
                }

                ui.separator();
                ui.label(egui::RichText::new("Find").color(TERMINAL_MUTED).size(12.0));
                ui.add(
                    egui::TextEdit::singleline(&mut tab.find_query)
                        .desired_width(140.0)
                        .hint_text("in file"),
                );

                if !tab.find_query.is_empty() {
                    let count = tab.draft.matches(&tab.find_query).count();
                    ui.label(
                        egui::RichText::new(format!("{count} matches"))
                            .color(TERMINAL_MUTED)
                            .size(12.0),
                    );
                }
            });
        });
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

fn show_edit_document(ui: &mut egui::Ui, tab: &mut OpenTab, kind: TextKind) {
    let before = tab.draft.clone();
    let width = ui.available_width().max(240.0);
    let height = (ui.available_height() - 86.0).max(220.0);
    let number_width = line_number_width(&tab.draft);
    let text_width = (width - number_width - 34.0).max(120.0);
    let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
        let wrap_width = wrap_width.min(text_width).max(120.0);
        let job = match kind {
            TextKind::Code(language) => highlighted_code_job_with_width(wrap_width, text, language),
            TextKind::Markdown | TextKind::Plain => egui::text::LayoutJob::simple(
                text.to_owned(),
                egui::FontId::monospace(EDITOR_FONT_SIZE),
                TERMINAL_TEXT,
                wrap_width,
            ),
        };

        ui.fonts(|fonts| fonts.layout_job(job))
    };

    egui::Frame::default()
        .fill(egui::Color32::from_rgb(8, 8, 9))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(54, 54, 58)))
        .inner_margin(8)
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(height);
            egui::ScrollArea::vertical()
                .id_salt("open_file_text_display")
                .auto_shrink([false, false])
                .max_height(height)
                .min_scrolled_height(height)
                .show(ui, |ui| {
                    let editor_font = egui::FontId::monospace(EDITOR_FONT_SIZE);
                    let row_height = ui.fonts(|fonts| fonts.row_height(&editor_font));
                    let line_count = line_count(&tab.draft) as f32;
                    let editor_height =
                        height.max((line_count + 1.0) * row_height + (EDITOR_TEXT_MARGIN_Y * 2.0));
                    let response = ui
                        .horizontal_top(|ui| {
                            let (number_rect, _) = ui.allocate_exact_size(
                                egui::vec2(number_width, editor_height),
                                egui::Sense::hover(),
                            );

                            let response = ui.add_sized(
                                egui::vec2(text_width, editor_height),
                                egui::TextEdit::multiline(&mut tab.draft)
                                    .id_salt(("file_editor", &tab.path))
                                    .code_editor()
                                    .margin(egui::Margin::symmetric(
                                        EDITOR_TEXT_MARGIN_X as i8,
                                        EDITOR_TEXT_MARGIN_Y as i8,
                                    ))
                                    .desired_width(text_width)
                                    .layouter(&mut layouter)
                                    .hint_text("Empty file"),
                            );

                            paint_line_numbers(
                                ui,
                                number_rect,
                                response.rect.top() + EDITOR_TEXT_MARGIN_Y,
                                row_height,
                                line_count as usize,
                                editor_font,
                            );

                            response
                        })
                        .inner;

                    if response.changed() || tab.draft != before {
                        tab.dirty = true;
                    }
                });
        });
}

fn compact_button(label: &str) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(label).size(12.0)).min_size(egui::vec2(46.0, 18.0))
}

fn paint_line_numbers(
    ui: &egui::Ui,
    number_rect: egui::Rect,
    text_top: f32,
    row_height: f32,
    line_count: usize,
    font: egui::FontId,
) {
    let painter = ui.painter_at(number_rect);
    let x = number_rect.right() - EDITOR_TEXT_MARGIN_X;

    for line in 1..=line_count {
        painter.text(
            egui::pos2(x, text_top + ((line - 1) as f32 * row_height)),
            egui::Align2::RIGHT_TOP,
            line.to_string(),
            font.clone(),
            TERMINAL_MUTED,
        );
    }
}

fn line_number_width(text: &str) -> f32 {
    let digits = line_count(text).to_string().len() as f32;
    (digits * 8.0 + 16.0).max(32.0)
}

fn line_count(text: &str) -> usize {
    text.split('\n').count().max(1)
}
