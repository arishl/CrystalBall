use std::path::Path;

use eframe::egui;

pub const TERMINAL_BG: egui::Color32 = egui::Color32::from_rgb(18, 20, 24);
pub const TERMINAL_PANEL_BG: egui::Color32 = egui::Color32::from_rgb(24, 27, 32);
pub const TERMINAL_TEXT: egui::Color32 = egui::Color32::from_rgb(220, 225, 232);
pub const TERMINAL_MUTED: egui::Color32 = egui::Color32::from_rgb(145, 153, 164);
pub const TERMINAL_ACCENT: egui::Color32 = egui::Color32::from_rgb(105, 190, 150);
pub const TERMINAL_ERROR: egui::Color32 = egui::Color32::from_rgb(230, 100, 100);

#[derive(Default)]
pub struct TerminalState {
    input: String,
    lines: Vec<TerminalLine>,
    should_scroll_to_bottom: bool,
}

impl TerminalState {
    pub fn push_command(&mut self, cwd: &Path, command: &str) {
        self.lines.push(TerminalLine::Command {
            cwd: cwd.display().to_string(),
            command: command.to_owned(),
        });
        self.should_scroll_to_bottom = true;
    }

    pub fn push_output(&mut self, output: impl Into<String>) {
        self.lines.push(TerminalLine::Output(output.into()));
        self.should_scroll_to_bottom = true;
    }

    pub fn push_error(&mut self, error: impl Into<String>) {
        self.lines.push(TerminalLine::Error(error.into()));
        self.should_scroll_to_bottom = true;
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.should_scroll_to_bottom = true;
    }
}

pub fn show_terminal(
    ui: &mut egui::Ui,
    terminal: &mut TerminalState,
    cwd: &Path,
) -> Option<String> {
    let mut command_to_run = None;
    let full_width = ui.available_width();

    egui::Frame::default()
        .fill(TERMINAL_PANEL_BG)
        .inner_margin(egui::Margin {
            left: 10,
            right: 10,
            top: 10,
            bottom: 16,
        })
        .show(ui, |ui| {
            ui.set_width(full_width);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Terminal")
                        .strong()
                        .color(TERMINAL_TEXT),
                );
                ui.label(
                    egui::RichText::new(cwd.display().to_string())
                        .monospace()
                        .color(TERMINAL_MUTED),
                );
            });

            ui.add_space(6.0);

            let prompt_height = ui.spacing().interact_size.y + 24.0;
            let output_height = (ui.available_height() - prompt_height - 32.0).max(24.0);

            egui::Frame::default()
                .fill(TERMINAL_BG)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(full_width - 20.0);

                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .max_width(ui.available_width())
                        .max_height(output_height)
                        .min_scrolled_height(output_height)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                                for line in &terminal.lines {
                                    show_terminal_line(ui, line);
                                }

                                if terminal.should_scroll_to_bottom {
                                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                                    terminal.should_scroll_to_bottom = false;
                                }
                            });
                        });
                });

            ui.add_space(6.0);

            egui::Frame::default()
                .fill(TERMINAL_BG)
                .inner_margin(egui::Margin::symmetric(8, 7))
                .show(ui, |ui| {
                    ui.set_width(full_width - 20.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("$")
                                .monospace()
                                .strong()
                                .color(TERMINAL_ACCENT),
                        );
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut terminal.input)
                                .desired_width(f32::INFINITY)
                                .font(egui::TextStyle::Monospace),
                        );

                        let enter_pressed = ui.input(|input| input.key_pressed(egui::Key::Enter));
                        if response.lost_focus() && enter_pressed {
                            let command = terminal.input.trim().to_owned();
                            terminal.input.clear();

                            if !command.is_empty() {
                                command_to_run = Some(command);
                            }

                            response.request_focus();
                        }
                    });
                });
        });

    command_to_run
}

fn show_terminal_line(ui: &mut egui::Ui, line: &TerminalLine) {
    match line {
        TerminalLine::Command { cwd, command } => {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} $", compact_path(cwd)))
                        .monospace()
                        .strong()
                        .color(TERMINAL_ACCENT),
                );
                ui.label(
                    egui::RichText::new(command)
                        .monospace()
                        .color(TERMINAL_TEXT),
                );
            });
        }
        TerminalLine::Output(output) => {
            ui.add(
                egui::Label::new(egui::RichText::new(output).monospace().color(TERMINAL_TEXT))
                    .wrap(),
            );
        }
        TerminalLine::Error(error) => {
            ui.add(
                egui::Label::new(egui::RichText::new(error).monospace().color(TERMINAL_ERROR))
                    .wrap(),
            );
        }
    }
}

fn compact_path(path: &str) -> String {
    if let Some(home) = std::env::var_os("HOME").and_then(|home| home.into_string().ok()) {
        if path == home {
            return "~".to_owned();
        }

        if let Some(rest) = path.strip_prefix(&(home + "/")) {
            return format!("~/{rest}");
        }
    }

    path.to_owned()
}

enum TerminalLine {
    Command { cwd: String, command: String },
    Output(String),
    Error(String),
}
