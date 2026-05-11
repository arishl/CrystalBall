mod app;
mod file_content;
mod file_entry;
mod markdown_view;
mod syntax_highlight;
mod terminal;

use app::FileExplorerApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1040.0, 680.0]),
        ..Default::default()
    };

    eframe::run_native(
        "CrystalBall",
        options,
        Box::new(|_cc| Ok(Box::<FileExplorerApp>::default())),
    )
}
