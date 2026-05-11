use std::path::PathBuf;

use eframe::egui;

use crate::file_content::{TextDocument, TextKind};
use crate::markdown_view::show_markdown;
use crate::syntax_highlight::show_code;

#[derive(Clone)]
pub(crate) struct HoverPreview {
    pub(crate) path: PathBuf,
    pub(crate) title: String,
    pub(crate) anchor: egui::Pos2,
    pub(crate) source_rect: egui::Rect,
}

pub(crate) fn show_document_preview(
    ui: &mut egui::Ui,
    document: Result<Option<TextDocument>, String>,
) {
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

fn show_document(ui: &mut egui::Ui, document: &TextDocument) {
    match document.kind {
        TextKind::Code(language) => show_code(ui, &document.text, language),
        TextKind::Markdown => show_markdown(ui, &document.text),
        TextKind::Plain => {
            ui.add(egui::Label::new(egui::RichText::new(&document.text).monospace()).wrap());
        }
    }
}
