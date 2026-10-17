pub mod editor;
pub mod pin;
pub mod region_selector;
pub mod settings;
pub mod theme;
pub mod updater_ui;
pub mod wizard;

pub use editor::run_editor;
pub use pin::run_pin_window;
pub use region_selector::run_region_selector;
pub use settings::run_settings_window;
pub use theme::{apply_adwaita_theme, AdwaitaThemeMode};
pub use updater_ui::run_updater_window;
pub use wizard::run_wizard_window;

pub fn load_app_icon() -> Option<egui::IconData> {
    static ICON_BYTES: &[u8] = include_bytes!("../../packaging/assets/bendershot.png");
    if let Ok(img) = image::load_from_memory(ICON_BYTES) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        })
    } else {
        None
    }
}


