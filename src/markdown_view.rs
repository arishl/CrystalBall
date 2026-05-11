use eframe::egui;

use crate::file_content::{CodeLanguage, code_language_from_name};
use crate::syntax_highlight::show_code;

pub fn show_markdown(ui: &mut egui::Ui, markdown: &str) {
    let mut in_code_block = false;
    let mut code_block = String::new();
    let mut code_language = None;

    for line in markdown.lines() {
        if let Some(fence_info) = line.trim_start().strip_prefix("```") {
            if in_code_block {
                show_code_block(ui, &code_block, code_language);
                code_block.clear();
                code_language = None;
            } else {
                code_language = code_language_from_name(fence_info);
            }
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            code_block.push_str(line);
            code_block.push('\n');
            continue;
        }

        show_markdown_line(ui, line);
    }

    if !code_block.is_empty() {
        show_code_block(ui, &code_block, code_language);
    }
}

fn show_markdown_line(ui: &mut egui::Ui, line: &str) {
    let trimmed = line.trim();

    if trimmed.is_empty() {
        ui.add_space(8.0);
        return;
    }

    if trimmed == "---" || trimmed == "***" {
        ui.separator();
        return;
    }

    if let Some(heading) = trimmed.strip_prefix("### ") {
        ui.heading(egui::RichText::new(heading).size(18.0));
        return;
    }

    if let Some(heading) = trimmed.strip_prefix("## ") {
        ui.heading(egui::RichText::new(heading).size(22.0));
        return;
    }

    if let Some(heading) = trimmed.strip_prefix("# ") {
        ui.heading(egui::RichText::new(heading).size(28.0));
        return;
    }

    if let Some(quote) = trimmed.strip_prefix("> ") {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("|").weak());
            ui.label(egui::RichText::new(render_inline(quote)).italics());
        });
        return;
    }

    if let Some(item) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
    {
        ui.horizontal_wrapped(|ui| {
            ui.label("•");
            ui.label(render_inline(item));
        });
        return;
    }

    if trimmed.starts_with("    ") {
        show_code_block(ui, trimmed, None);
        return;
    }

    ui.label(render_inline(trimmed));
}

fn show_code_block(ui: &mut egui::Ui, code: &str, language: Option<CodeLanguage>) {
    if let Some(language) = language {
        show_code(ui, code, language);
    } else {
        egui::Frame::default()
            .fill(ui.visuals().extreme_bg_color)
            .inner_margin(8)
            .show(ui, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(code).monospace()).wrap());
            });
    }
}

fn render_inline(text: &str) -> String {
    // Keep inline Markdown readable even when this lightweight renderer
    // does not style every span separately.
    text.replace("**", "").replace("__", "").replace('`', "")
}
