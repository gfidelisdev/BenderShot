use eframe::egui::{self, Color32, Pos2, Rect};
use image::RgbaImage;

pub struct PinApp {
    image: RgbaImage,
    texture: Option<egui::TextureHandle>,
}

impl PinApp {
    pub fn new(image: RgbaImage) -> Self {
        Self {
            image,
            texture: None,
        }
    }
}

impl eframe::App for PinApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.texture.is_none() {
            let (w, h) = self.image.dimensions();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                self.image.as_raw(),
            );
            self.texture = Some(ctx.load_texture("pin_image", color_image, egui::TextureOptions::LINEAR));
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let size = ui.available_size();
                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());

                if let Some(texture) = &self.texture {
                    ui.painter().image(
                        texture.id(),
                        rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }

                // Allow dragging window by clicking anywhere on it
                if response.dragged() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            });
    }
}

pub fn run_pin_window(image: RgbaImage) -> Result<(), eframe::Error> {
    let width = (image.width() as f32).max(200.0);
    let height = (image.height() as f32).max(150.0);

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("BenderShot - Pinned")
        .with_app_id("bendershot")
        .with_always_on_top()
        .with_decorations(false)
        .with_transparent(true)
        .with_inner_size([width, height]);
    if let Some(icon) = crate::ui::load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "bendershot",
        options,
        Box::new(|_cc| Ok(Box::new(PinApp::new(image)))),
    )
}
