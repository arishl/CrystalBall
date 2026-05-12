use eframe::egui;

use crate::file_content::TextDocument;
use crate::terminal::TERMINAL_MUTED;

use super::preview::{HoverPreview, show_document_preview};

pub(crate) const PREVIEW_CLOSE_GRACE_SECONDS: f64 = 0.18;

const PREVIEW_WIDTH: f32 = 560.0;
const PREVIEW_HEIGHT: f32 = 360.0;
const PREVIEW_HOVER_PADDING: f32 = 4.0;
const PREVIEW_SOURCE_PADDING: f32 = 2.0;

pub(crate) fn show_hover_preview(
    ctx: &egui::Context,
    hover_preview: &mut Option<HoverPreview>,
    preview: HoverPreview,
    preview_row_hovered: bool,
    preview_keep_until: &mut f64,
    document: Result<Option<TextDocument>, String>,
) {
    let screen = ctx.screen_rect();
    let x = preview
        .anchor
        .x
        .min(screen.right() - PREVIEW_WIDTH - 12.0)
        .max(screen.left() + 12.0);
    let y = preview
        .anchor
        .y
        .min(screen.bottom() - PREVIEW_HEIGHT - 12.0)
        .max(screen.top() + 48.0);

    let inner = egui::Area::new("file_hover_preview".into())
        .order(egui::Order::Tooltip)
        .interactable(true)
        .fixed_pos(egui::pos2(x, y))
        .show(ctx, |ui| {
            egui::Frame::default()
                .fill(egui::Color32::from_rgb(18, 18, 20))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(54, 54, 58)))
                .corner_radius(8)
                .inner_margin(10)
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(PREVIEW_WIDTH, PREVIEW_HEIGHT));
                    ui.set_max_size(egui::vec2(PREVIEW_WIDTH, PREVIEW_HEIGHT));
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(preview.title.clone()).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("Preview").color(TERMINAL_MUTED));
                        });
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt(("hover_file_preview_scroll", &preview.path))
                        .auto_shrink([false, false])
                        .min_scrolled_height(PREVIEW_HEIGHT - 44.0)
                        .max_height(PREVIEW_HEIGHT - 44.0)
                        .show(ui, |ui| {
                            ui.set_width(PREVIEW_WIDTH - 24.0);
                            show_document_preview(ui, document);
                        });
                });
        });

    let pointer_in_preview = ctx.pointer_hover_pos().is_some_and(|pos| {
        inner
            .response
            .rect
            .expand(PREVIEW_HOVER_PADDING)
            .contains(pos)
    });
    let pointer_in_source = ctx.pointer_hover_pos().is_some_and(|pos| {
        preview
            .source_rect
            .expand(PREVIEW_SOURCE_PADDING)
            .contains(pos)
    });

    if preview_row_hovered || inner.response.hovered() || pointer_in_preview {
        *preview_keep_until = ctx.input(|input| input.time) + PREVIEW_CLOSE_GRACE_SECONDS;
    }

    if !preview_row_hovered
        && !inner.response.hovered()
        && !pointer_in_preview
        && !pointer_in_source
        && ctx.input(|input| input.time) > *preview_keep_until
    {
        *hover_preview = None;
    }
}
