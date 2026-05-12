use eframe::egui;

use crate::terminal::{
    TERMINAL_ACCENT, TERMINAL_BG, TERMINAL_ERROR, TERMINAL_MUTED, TERMINAL_PANEL_BG, TERMINAL_TEXT,
};

pub(crate) fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    let subtle_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(54, 54, 58));
    let active_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 220, 224));

    visuals.override_text_color = Some(TERMINAL_TEXT);
    visuals.panel_fill = TERMINAL_BG;
    visuals.window_fill = TERMINAL_PANEL_BG;
    visuals.extreme_bg_color = TERMINAL_BG;
    visuals.faint_bg_color = egui::Color32::from_rgb(26, 26, 28);
    visuals.code_bg_color = TERMINAL_BG;
    visuals.hyperlink_color = TERMINAL_ACCENT;
    visuals.error_fg_color = TERMINAL_ERROR;
    visuals.selection.bg_fill = egui::Color32::from_rgb(72, 72, 76);
    visuals.selection.stroke = egui::Stroke::new(1.0, TERMINAL_TEXT);
    visuals.window_stroke = subtle_stroke;
    visuals.widgets.noninteractive.bg_fill = TERMINAL_PANEL_BG;
    visuals.widgets.noninteractive.weak_bg_fill = TERMINAL_PANEL_BG;
    visuals.widgets.noninteractive.bg_stroke = subtle_stroke;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TERMINAL_TEXT);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(28, 28, 31);
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(28, 28, 31);
    visuals.widgets.inactive.bg_stroke = subtle_stroke;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TERMINAL_TEXT);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(44, 44, 48);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(44, 44, 48);
    visuals.widgets.hovered.bg_stroke = active_stroke;
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TERMINAL_TEXT);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(76, 76, 82);
    visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(76, 76, 82);
    visuals.widgets.active.bg_stroke = active_stroke;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, TERMINAL_TEXT);
    visuals.widgets.open.bg_fill = egui::Color32::from_rgb(44, 44, 48);
    visuals.widgets.open.weak_bg_fill = egui::Color32::from_rgb(44, 44, 48);
    visuals.widgets.open.bg_stroke = active_stroke;
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, TERMINAL_TEXT);
    visuals.warn_fg_color = TERMINAL_MUTED;
    visuals.button_frame = true;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.window_margin = egui::Margin::same(10);
    ctx.set_style(style);
}

pub(crate) struct StatusMessage {
    pub(crate) text: String,
    is_error: bool,
}

impl StatusMessage {
    pub(crate) fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: false,
        }
    }

    pub(crate) fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: true,
        }
    }

    pub(crate) fn color(&self) -> egui::Color32 {
        if self.is_error {
            TERMINAL_ERROR
        } else {
            TERMINAL_ACCENT
        }
    }
}
