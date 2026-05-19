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

    fn diamond_alpha(x: f32, y: f32, cx: f32, cy: f32, rx: f32, ry: f32) -> f32 {
        let distance = (x - cx).abs() / rx + (y - cy).abs() / ry;
        ((1.0 - distance) * SIZE as f32 * 0.22).clamp(0.0, 1.0)
    }

    fn band_alpha(y: f32, center: f32, width: f32) -> f32 {
        ((width - (y - center).abs()) * SIZE as f32).clamp(0.0, 1.0)
    }

    fn base_alpha(x: f32, y: f32) -> f32 {
        if !(0.66..=0.84).contains(&y) {
            return 0.0;
        }

        let top_width = 0.34;
        let bottom_width = 0.48;
        let t = ((y - 0.66) / 0.18).clamp(0.0, 1.0);
        let half_width = top_width / 2.0 + (bottom_width - top_width) * t / 2.0;
        ((half_width - (x - 0.5).abs()) * SIZE as f32).clamp(0.0, 1.0)
    }

    fn pixel(x: f32, y: f32) -> [u8; 4] {
        let bg_alpha = rounded_rect_alpha(x, y, 0.22);
        let mut color = [36, 16, 60, clamp(255.0 * bg_alpha)];

        for center in [0.22, 0.38, 0.54, 0.70] {
            let band = band_alpha(y, center, 0.012);
            if band > 0.0 {
                color = mix(color, [22, 7, 35, color[3]], band * 0.45);
            }
        }

        let border = if x < 0.035 || x > 0.965 || y < 0.035 || y > 0.965 {
            bg_alpha
        } else {
            0.0
        };
        if border > 0.0 {
            color = mix(color, [246, 214, 109, 255], border * 0.88);
        }

        let fx = (x - 0.5) / 0.82 + 0.5;
        let fy = (y - 0.5) / 0.82 + 0.5;

        for (cx, cy, rx, ry, star_color) in [
            (0.215, 0.225, 0.085, 0.085, [255, 227, 122, 255]),
            (0.785, 0.225, 0.065, 0.065, [255, 209, 90, 255]),
        ] {
            let sparkle = diamond_alpha(fx, fy, cx, cy, rx, ry);
            if sparkle > 0.0 {
                color = mix(color, star_color, sparkle);
            }
        }

        let globe = circle_alpha(fx, fy, 0.5, 0.42, 0.285);
        if globe > 0.0 {
            color = mix(color, [142, 75, 197, 235], globe);
        }

        let rim = ((fx - 0.5).hypot(fy - 0.42) - 0.285).abs();
        if rim < 0.024 {
            color = mix(color, [255, 233, 163, 255], 1.0 - rim / 0.024);
        }

        let highlight = circle_alpha(fx, fy, 0.40, 0.31, 0.055);
        if highlight > 0.0 {
            color = mix(color, [255, 246, 207, 255], highlight * 0.92);
        }

        let base = base_alpha(fx, fy);
        if base > 0.0 {
            color = mix(color, [57, 32, 78, 255], base);
        }

        let base_top = band_alpha(fy, 0.66, 0.014);
        if base_top > 0.0 && (fx - 0.5).abs() < 0.18 {
            color = mix(color, [246, 214, 109, 255], base_top);
        }

        let base_groove = band_alpha(fy, 0.79, 0.010);
        if base_groove > 0.0 && (fx - 0.5).abs() < 0.30 {
            color = mix(color, [184, 128, 56, 255], base_groove);
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
