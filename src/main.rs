mod app;
mod auto_update;
mod file_content;
mod file_entry;
mod markdown_view;
mod syntax_highlight;
mod terminal;

use app::FileExplorerApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 680.0])
            .with_icon(app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "CrystalBall",
        options,
        Box::new(|_cc| Ok(Box::<FileExplorerApp>::default())),
    )
}

fn app_icon() -> egui::IconData {
    const SIZE: usize = 128;

    fn clamp(value: f32) -> u8 {
        value.round().clamp(0.0, 255.0) as u8
    }

    fn mix(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
        [
            clamp(a[0] as f32 + (b[0] as f32 - a[0] as f32) * t),
            clamp(a[1] as f32 + (b[1] as f32 - a[1] as f32) * t),
            clamp(a[2] as f32 + (b[2] as f32 - a[2] as f32) * t),
            clamp(a[3] as f32 + (b[3] as f32 - a[3] as f32) * t),
        ]
    }

    fn rounded_rect_alpha(x: f32, y: f32, radius: f32) -> f32 {
        let px = (x - 0.5).abs();
        let py = (y - 0.5).abs();
        let inner = 0.5 - radius;
        let dx = (px - inner).max(0.0);
        let dy = (py - inner).max(0.0);
        ((radius - dx.hypot(dy)) * SIZE as f32).clamp(0.0, 1.0)
    }

    fn circle_alpha(x: f32, y: f32, cx: f32, cy: f32, radius: f32) -> f32 {
        ((radius - (x - cx).hypot(y - cy)) * SIZE as f32).clamp(0.0, 1.0)
    }

    fn line_alpha(x: f32, y: f32, x1: f32, y1: f32, x2: f32, y2: f32, width: f32) -> f32 {
        let vx = x2 - x1;
        let vy = y2 - y1;
        let wx = x - x1;
        let wy = y - y1;
        let length_sq = vx * vx + vy * vy;
        let t = if length_sq == 0.0 {
            0.0
        } else {
            ((wx * vx + wy * vy) / length_sq).clamp(0.0, 1.0)
        };
        let distance = (x - (x1 + t * vx)).hypot(y - (y1 + t * vy));
        ((width - distance) * SIZE as f32).clamp(0.0, 1.0)
    }

    fn pixel(x: f32, y: f32) -> [u8; 4] {
        let bg_alpha = rounded_rect_alpha(x, y, 0.22);
        let mut color = [8, 8, 9, clamp(255.0 * bg_alpha)];

        let base_top = y > 0.72 && (x - 0.5).abs() < 0.25 + (y - 0.72) * 0.75;
        if base_top {
            let shade = 22.0 + 54.0 * (1.0 - (x - 0.5).abs() * 2.0);
            color = mix(
                color,
                [clamp(shade), clamp(shade), clamp(shade + 4.0), 255],
                bg_alpha,
            );
        }

        let globe = circle_alpha(x, y, 0.5, 0.42, 0.285);
        if globe > 0.0 {
            let light = (1.0 - (x - 0.38).hypot(y - 0.29) / 0.45).max(0.0);
            let edge = (x - 0.5).hypot(y - 0.42) / 0.285;
            let gray = clamp(148.0 + 92.0 * light + 40.0 * (edge - 0.72).max(0.0));
            color = mix(color, [gray, gray, gray.saturating_add(4), 228], globe);
        }

        let rim = ((x - 0.5).hypot(y - 0.42) - 0.285).abs();
        if rim < 0.012 {
            color = mix(color, [252, 252, 253, 255], 1.0 - rim / 0.012);
        }

        let highlight = line_alpha(x, y, 0.36, 0.29, 0.58, 0.19, 0.028);
        if highlight > 0.0 {
            color = mix(color, [255, 255, 255, 255], highlight * 0.88);
        }

        let shadow = line_alpha(x, y, 0.35, 0.56, 0.64, 0.59, 0.018);
        if shadow > 0.0 {
            color = mix(color, [24, 24, 27, 255], shadow * 0.35);
        }

        color
    }

    let mut rgba = Vec::with_capacity(SIZE * SIZE * 4);
    for py in 0..SIZE {
        for px in 0..SIZE {
            let x = (px as f32 + 0.5) / SIZE as f32;
            let y = (py as f32 + 0.5) / SIZE as f32;
            rgba.extend(pixel(x, y));
        }
    }

    egui::IconData {
        rgba,
        width: SIZE as u32,
        height: SIZE as u32,
    }
}
