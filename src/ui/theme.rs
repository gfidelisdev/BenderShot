use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Margin, Rounding, Stroke, Vec2, Visuals};
use std::sync::Once;

static FONT_INIT: Once = Once::new();

pub fn setup_fonts(ctx: &egui::Context) {
    FONT_INIT.call_once(|| {
        let mut fonts = FontDefinitions::default();

        // 1. Try loading native Adwaita / Cantarell UI fonts for standard text
        let ui_font_candidates = [
            ("/usr/share/fonts/adwaita-sans-fonts/AdwaitaSans-Regular.ttf", "adwaita_sans"),
            ("/usr/share/fonts/abattis-cantarell-vf-fonts/Cantarell-VF.otf", "cantarell"),
            ("/usr/share/fonts/google-noto/NotoSans-Regular.ttf", "noto_sans"),
            ("C:\\Windows\\Fonts\\segoeui.ttf", "segoe_ui"),
            ("C:\\Windows\\Fonts\\arial.ttf", "arial"),
        ];

        for (path, name) in ui_font_candidates {
            if let Ok(bytes) = std::fs::read(path) {
                fonts.font_data.insert(name.to_owned(), FontData::from_owned(bytes));
                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .insert(0, name.to_owned());
                break;
            }
        }

        // 2. Load system symbol/emoji fonts as fallbacks for icons (Symbola, NotoEmoji, NotoSansSymbols, AdwaitaMono)
        let symbol_font_candidates = [
            ("/usr/share/fonts/gdouros-symbola/Symbola.ttf", "symbola"),
            ("/usr/share/fonts/google-noto-emoji-fonts/NotoEmoji-Regular.ttf", "noto_emoji"),
            ("/usr/share/fonts/adwaita-mono-fonts/AdwaitaMono-Regular.ttf", "adwaita_mono"),
            ("/usr/share/fonts/google-noto/NotoSansSymbols2-Regular.ttf", "noto_symbols"),
            ("C:\\Windows\\Fonts\\seguiemj.ttf", "segoe_emoji"),
            ("C:\\Windows\\Fonts\\seguisym.ttf", "segoe_sym"),
        ];

        for (path, name) in symbol_font_candidates {
            if let Ok(bytes) = std::fs::read(path) {
                fonts.font_data.insert(name.to_owned(), FontData::from_owned(bytes));
                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .push(name.to_owned());
                fonts
                    .families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .push(name.to_owned());
            }
        }

        ctx.set_fonts(fonts);
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdwaitaThemeMode {
    Dark,
    Light,
}

pub fn apply_adwaita_theme(ctx: &egui::Context, mode: AdwaitaThemeMode) {
    setup_fonts(ctx);

    let mut style = (*ctx.style()).clone();

    // Comfortable GNOME spacing
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.window_margin = Margin::same(12.0);

    let mut visuals = match mode {
        AdwaitaThemeMode::Dark => {
            let mut v = Visuals::dark();
            v.panel_fill = Color32::from_rgb(36, 36, 36);      // #242424
            v.window_fill = Color32::from_rgb(46, 46, 46);     // #2e2e2e
            v.extreme_bg_color = Color32::from_rgb(30, 30, 30);

            // Widgets noninteractive (headers, backgrounds)
            v.widgets.noninteractive.bg_fill = Color32::from_rgb(42, 42, 42);
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 60));
            v.widgets.noninteractive.rounding = Rounding::same(8.0);
            v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(240, 240, 240));

            // Widgets inactive (default buttons)
            v.widgets.inactive.bg_fill = Color32::from_rgb(56, 56, 56); // #383838
            v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(70, 70, 70));
            v.widgets.inactive.rounding = Rounding::same(8.0);
            v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(235, 235, 235));

            // Widgets hovered
            v.widgets.hovered.bg_fill = Color32::from_rgb(68, 68, 68);
            v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(90, 90, 90));
            v.widgets.hovered.rounding = Rounding::same(8.0);
            v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

            // Widgets active / pressed
            v.widgets.active.bg_fill = Color32::from_rgb(53, 132, 228); // Adwaita blue
            v.widgets.active.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(53, 132, 228));
            v.widgets.active.rounding = Rounding::same(8.0);
            v.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

            // Widgets open / selected
            v.widgets.open.bg_fill = Color32::from_rgb(53, 132, 228);
            v.widgets.open.rounding = Rounding::same(8.0);

            v.selection.bg_fill = Color32::from_rgb(53, 132, 228);
            v
        }
        AdwaitaThemeMode::Light => {
            let mut v = Visuals::light();
            v.panel_fill = Color32::from_rgb(250, 250, 250);   // #fafafa
            v.window_fill = Color32::from_rgb(255, 255, 255);
            v.extreme_bg_color = Color32::from_rgb(242, 242, 242);

            v.widgets.noninteractive.bg_fill = Color32::from_rgb(245, 245, 245);
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(220, 220, 220));
            v.widgets.noninteractive.rounding = Rounding::same(8.0);
            v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(40, 40, 40));

            v.widgets.inactive.bg_fill = Color32::from_rgb(238, 238, 238);
            v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(215, 215, 215));
            v.widgets.inactive.rounding = Rounding::same(8.0);
            v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(30, 30, 30));

            v.widgets.hovered.bg_fill = Color32::from_rgb(228, 228, 228);
            v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(195, 195, 195));
            v.widgets.hovered.rounding = Rounding::same(8.0);
            v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(10, 10, 10));

            v.widgets.active.bg_fill = Color32::from_rgb(28, 113, 216);
            v.widgets.active.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(28, 113, 216));
            v.widgets.active.rounding = Rounding::same(8.0);
            v.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

            v.widgets.open.bg_fill = Color32::from_rgb(28, 113, 216);
            v.widgets.open.rounding = Rounding::same(8.0);

            v.selection.bg_fill = Color32::from_rgb(28, 113, 216);
            v
        }
    };

    visuals.window_rounding = Rounding::same(12.0);
    style.visuals = visuals;
    ctx.set_style(style);
}
