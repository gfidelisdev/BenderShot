use crate::export::Exporter;
use crate::model::{AnnotationItem, ObfuscateKind, Scene, ToolMode};
use crate::rasterize::{apply_obfuscation, apply_smudge_step, rasterize_scene};
use crate::ui::theme::{apply_adwaita_theme, AdwaitaThemeMode};
use eframe::egui::{self, Color32, Frame, Margin, Pos2, Rect, Rounding, Stroke, Vec2};
use image::RgbaImage;
use std::sync::mpsc::Sender;

enum UndoEntry {
    VectorItem,
    RasterSnapshot(RgbaImage),
}

struct InlineTextState {
    pos: [f32; 2],
    text: String,
}

pub struct EditorApp {
    base_image: RgbaImage,
    undo_history: Vec<UndoEntry>,
    redo_history: Vec<UndoEntry>,
    texture: Option<egui::TextureHandle>,
    scene: Scene,
    current_tool: ToolMode,
    current_color: [u8; 4],
    stroke_width: f32,
    drag_start: Option<Pos2>,
    drag_current: Option<Pos2>,
    active_inline_text: Option<InlineTextState>,
    show_loupe: bool,
    theme_mode: AdwaitaThemeMode,
    smudge_carried_color: Option<[f32; 4]>,
    status_message: Option<(String, std::time::Instant)>,
    pin_sender: Option<Sender<RgbaImage>>,
}

impl EditorApp {
    pub fn new(base_image: RgbaImage, pin_sender: Option<Sender<RgbaImage>>) -> Self {
        Self {
            base_image,
            undo_history: Vec::new(),
            redo_history: Vec::new(),
            texture: None,
            scene: Scene::new(),
            current_tool: ToolMode::Arrow,
            current_color: [224, 27, 36, 255], // Adwaita Red
            stroke_width: 4.0,
            drag_start: None,
            drag_current: None,
            active_inline_text: None,
            show_loupe: true,
            theme_mode: AdwaitaThemeMode::Dark,
            smudge_carried_color: None,
            status_message: None,
            pin_sender,
        }
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }

    fn color32(&self) -> Color32 {
        Color32::from_rgba_unmultiplied(
            self.current_color[0],
            self.current_color[1],
            self.current_color[2],
            self.current_color[3],
        )
    }

    fn undo(&mut self) {
        if let Some(entry) = self.undo_history.pop() {
            match entry {
                UndoEntry::VectorItem => {
                    self.scene.undo();
                    self.redo_history.push(UndoEntry::VectorItem);
                }
                UndoEntry::RasterSnapshot(prev_img) => {
                    self.redo_history.push(UndoEntry::RasterSnapshot(self.base_image.clone()));
                    self.base_image = prev_img;
                    self.texture = None;
                }
            }
            self.set_status("Desfeito");
        }
    }

    fn redo(&mut self) {
        if let Some(entry) = self.redo_history.pop() {
            match entry {
                UndoEntry::VectorItem => {
                    self.scene.redo();
                    self.undo_history.push(UndoEntry::VectorItem);
                }
                UndoEntry::RasterSnapshot(next_img) => {
                    self.undo_history.push(UndoEntry::RasterSnapshot(self.base_image.clone()));
                    self.base_image = next_img;
                    self.texture = None;
                }
            }
            self.set_status("Refeito");
        }
    }

    fn commit_inline_text(&mut self) {
        if let Some(inline) = self.active_inline_text.take() {
            let trimmed = inline.text.trim();
            if !trimmed.is_empty() {
                self.scene.push_item(AnnotationItem::Text {
                    pos: inline.pos,
                    text: trimmed.to_string(),
                    color: self.current_color,
                    font_size: (self.stroke_width * 6.0).max(24.0),
                });
                self.undo_history.push(UndoEntry::VectorItem);
                self.redo_history.clear();
                self.set_status("Texto inserido");
            }
        }
    }

    fn finish_current_shape(&mut self, start: Pos2, mut end: Pos2, image_rect: Rect, is_ctrl_constrained: bool) {
        let to_image_coords = |p: Pos2| -> [f32; 2] {
            let x = (p.x - image_rect.min.x).clamp(0.0, image_rect.width());
            let y = (p.y - image_rect.min.y).clamp(0.0, image_rect.height());
            [x, y]
        };

        if is_ctrl_constrained && matches!(self.current_tool, ToolMode::Rectangle | ToolMode::Ellipse | ToolMode::Crop) {
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let size = dx.abs().max(dy.abs());
            end = Pos2::new(start.x + dx.signum() * size, start.y + dy.signum() * size);
        }

        let p1 = to_image_coords(start);
        let p2 = to_image_coords(end);

        match self.current_tool {
            ToolMode::Arrow => {
                if (start - end).length() > 5.0 {
                    self.scene.push_item(AnnotationItem::Arrow {
                        start: p1,
                        end: p2,
                        color: self.current_color,
                        width: self.stroke_width,
                    });
                    self.undo_history.push(UndoEntry::VectorItem);
                    self.redo_history.clear();
                }
            }
            ToolMode::Rectangle => {
                let min = [p1[0].min(p2[0]), p1[1].min(p2[1])];
                let max = [p1[0].max(p2[0]), p1[1].max(p2[1])];
                if (max[0] - min[0]) > 4.0 && (max[1] - min[1]) > 4.0 {
                    self.scene.push_item(AnnotationItem::Rectangle {
                        min,
                        max,
                        color: self.current_color,
                        width: self.stroke_width,
                        fill: None,
                    });
                    self.undo_history.push(UndoEntry::VectorItem);
                    self.redo_history.clear();
                }
            }
            ToolMode::Ellipse => {
                let min = [p1[0].min(p2[0]), p1[1].min(p2[1])];
                let max = [p1[0].max(p2[0]), p1[1].max(p2[1])];
                if (max[0] - min[0]) > 4.0 && (max[1] - min[1]) > 4.0 {
                    self.scene.push_item(AnnotationItem::Ellipse {
                        min,
                        max,
                        color: self.current_color,
                        width: self.stroke_width,
                        fill: None,
                    });
                    self.undo_history.push(UndoEntry::VectorItem);
                    self.redo_history.clear();
                }
            }
            ToolMode::Blur => {
                let min = [p1[0].min(p2[0]), p1[1].min(p2[1])];
                let max = [p1[0].max(p2[0]), p1[1].max(p2[1])];
                if (max[0] - min[0]) > 2.0 && (max[1] - min[1]) > 2.0 {
                    self.undo_history.push(UndoEntry::RasterSnapshot(self.base_image.clone()));
                    self.redo_history.clear();
                    apply_obfuscation(
                        &mut self.base_image,
                        min,
                        max,
                        ObfuscateKind::Blur,
                        (self.stroke_width * 3.0) as u32,
                    );
                    self.texture = None;
                    self.set_status("Blur aplicado");
                }
            }
            ToolMode::Pixelate => {
                let min = [p1[0].min(p2[0]), p1[1].min(p2[1])];
                let max = [p1[0].max(p2[0]), p1[1].max(p2[1])];
                if (max[0] - min[0]) > 2.0 && (max[1] - min[1]) > 2.0 {
                    self.undo_history.push(UndoEntry::RasterSnapshot(self.base_image.clone()));
                    self.redo_history.clear();
                    apply_obfuscation(
                        &mut self.base_image,
                        min,
                        max,
                        ObfuscateKind::Pixelate,
                        (self.stroke_width * 3.0) as u32,
                    );
                    self.texture = None;
                    self.set_status("Pixelização aplicada");
                }
            }
            ToolMode::Crop => {
                let min_x = p1[0].min(p2[0]).max(0.0) as u32;
                let min_y = p1[1].min(p2[1]).max(0.0) as u32;
                let max_x = (p1[0].max(p2[0]) as u32).min(self.base_image.width());
                let max_y = (p1[1].max(p2[1]) as u32).min(self.base_image.height());

                if max_x > min_x + 10 && max_y > min_y + 10 {
                    self.undo_history.push(UndoEntry::RasterSnapshot(self.base_image.clone()));
                    self.redo_history.clear();
                    let cropped = image::imageops::crop_imm(
                        &self.base_image,
                        min_x,
                        min_y,
                        max_x - min_x,
                        max_y - min_y,
                    )
                    .to_image();
                    self.base_image = cropped;
                    self.texture = None;
                    self.scene.clear();
                    self.current_tool = ToolMode::Select;
                    self.set_status("Imagem recortada com sucesso");
                }
            }
            _ => {}
        }
    }

    fn export_current_raster(&self) -> RgbaImage {
        rasterize_scene(&self.base_image, &self.scene)
    }

    fn copy_and_exit(&mut self, ctx: &egui::Context) {
        self.commit_inline_text();
        let final_image = self.export_current_raster();
        if let Err(e) = Exporter::copy_to_clipboard(&final_image) {
            tracing::error!("Failed to copy to clipboard: {:#}", e);
            self.set_status(format!("Erro ao copiar: {e}"));
        } else {
            let _ = Exporter::auto_save(&final_image);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Enforce Libadwaita theme styling
        apply_adwaita_theme(ctx, self.theme_mode);

        // Initialize or update texture
        if self.texture.is_none() {
            let (w, h) = self.base_image.dimensions();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                self.base_image.as_raw(),
            );
            self.texture = Some(ctx.load_texture("screenshot", color_image, egui::TextureOptions::LINEAR));
        }

        // Global keyboard shortcuts
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::C)) {
            self.copy_and_exit(ctx);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
            self.undo();
        }
        if ctx.input(|i| {
            (i.modifiers.command && i.key_pressed(egui::Key::Y))
                || (i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Z))
        }) {
            self.redo();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.active_inline_text.is_some() {
                self.active_inline_text = None;
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        let is_ctrl = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);

        // Top HeaderBar (GNOME Libadwaita Style)
        let header_bg = match self.theme_mode {
            AdwaitaThemeMode::Dark => Color32::from_rgb(46, 46, 46),
            AdwaitaThemeMode::Light => Color32::from_rgb(255, 255, 255),
        };

        egui::TopBottomPanel::top("adwaita_headerbar")
            .frame(Frame::none().fill(header_bg).inner_margin(Margin::symmetric(14.0, 10.0)))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    // App Title & Badge
                    ui.label(egui::RichText::new("📷 Enhanced Shot").strong().size(15.0));
                    ui.separator();

                    let next_badge = self.scene.next_badge_number;
                    // Pill Segmented Tool Group
                    ui.group(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                        let mut tool_btn = |mode: ToolMode, label: &str| {
                            let selected = self.current_tool == mode;
                            if ui.selectable_label(selected, label).clicked() {
                                self.commit_inline_text();
                                self.current_tool = mode;
                            }
                        };

                        tool_btn(ToolMode::Select, "↖");
                        tool_btn(ToolMode::Arrow, "↗ Seta");
                        tool_btn(ToolMode::Rectangle, "▢ Box");
                        tool_btn(ToolMode::Ellipse, "◯ Elipse");
                        tool_btn(ToolMode::Text, "T Texto");
                        tool_btn(ToolMode::StepBadge, &format!("① Passo ({})", next_badge));
                        tool_btn(ToolMode::Blur, "░ Blur");
                        tool_btn(ToolMode::Pixelate, "▦ Pixel");
                        tool_btn(ToolMode::Smudge, "🖐 Smudge");
                        tool_btn(ToolMode::Crop, "⯏ Cortar");
                    });

                    ui.separator();

                    // Stroke width
                    ui.label("Traço:");
                    ui.add(egui::Slider::new(&mut self.stroke_width, 1.0..=20.0).show_value(false));

                    // Color palette pills
                    let palette: [[u8; 4]; 7] = [
                        [224, 27, 36, 255],   // Adwaita Red
                        [255, 120, 0, 255],   // Adwaita Orange
                        [246, 211, 45, 255],  // Adwaita Yellow
                        [46, 194, 126, 255],  // Adwaita Green
                        [53, 132, 228, 255],  // Adwaita Blue
                        [255, 255, 255, 255], // White
                        [20, 20, 20, 255],    // Black
                    ];

                    for color in palette {
                        let c32 = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::click());
                        ui.painter().rect_filled(rect, Rounding::same(9.0), c32);
                        if self.current_color == color {
                            ui.painter().rect_stroke(rect, Rounding::same(9.0), Stroke::new(2.0_f32, Color32::WHITE));
                        }
                        if resp.clicked() {
                            self.current_color = color;
                        }
                    }

                    ui.separator();

                    // Undo / Redo
                    if ui.button("↶").on_hover_text("Desfazer (Ctrl+Z)").clicked() {
                        self.undo();
                    }
                    if ui.button("↷").on_hover_text("Refazer (Ctrl+Shift+Z)").clicked() {
                        self.redo();
                    }

                    ui.separator();

                    // Action Buttons (with Adwaita Blue primary button for Copy)
                    let copy_btn = egui::Button::new(
                        egui::RichText::new("📋 Copiar (Ctrl+C)")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(53, 132, 228)) // Adwaita Blue
                    .rounding(Rounding::same(8.0));

                    if ui.add(copy_btn).clicked() {
                        self.copy_and_exit(ctx);
                    }

                    if ui.button("💾 Salvar").clicked() {
                        self.commit_inline_text();
                        let final_img = self.export_current_raster();
                        if let Ok(saved_path) = Exporter::auto_save(&final_img) {
                            self.set_status(format!("Salvo em: {:?}", saved_path));
                        }
                    }

                    if ui.button("📌 Fixar").on_hover_text("Manter screenshot flutuando no topo da tela").clicked() {
                        self.commit_inline_text();
                        let final_img = self.export_current_raster();
                        if let Some(ref sender) = self.pin_sender {
                            let _ = sender.send(final_img);
                        }
                        self.set_status("Screenshot fixado na tela");
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Dark / Light Theme Toggle
                        let theme_icon = match self.theme_mode {
                            AdwaitaThemeMode::Dark => "☀️ Claro",
                            AdwaitaThemeMode::Light => "🌙 Escuro",
                        };
                        if ui.button(theme_icon).clicked() {
                            self.theme_mode = match self.theme_mode {
                                AdwaitaThemeMode::Dark => AdwaitaThemeMode::Light,
                                AdwaitaThemeMode::Light => AdwaitaThemeMode::Dark,
                            };
                        }
                    });
                });
            });

        // Bottom Status Toast
        egui::TopBottomPanel::bottom("adwaita_footer")
            .frame(Frame::none().fill(header_bg).inner_margin(Margin::symmetric(14.0, 6.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!(
                        "{}x{} px | {} camadas",
                        self.base_image.width(),
                        self.base_image.height(),
                        self.scene.items.len()
                    )).weak().size(12.0));

                    ui.separator();
                    ui.checkbox(&mut self.show_loupe, "🔍 Lupa");

                    if self.current_tool == ToolMode::Text {
                        ui.separator();
                        ui.label(egui::RichText::new("👉 Clique em qualquer ponto da tela para digitar o texto diretamente.").size(12.0));
                    } else if self.current_tool == ToolMode::StepBadge {
                        ui.separator();
                        ui.label(egui::RichText::new("👉 Clique diretamente para estampar o próximo passo.").size(12.0));
                    } else if matches!(self.current_tool, ToolMode::Ellipse | ToolMode::Rectangle) {
                        ui.separator();
                        ui.label(egui::RichText::new("💡 Dica: Segure Ctrl para travar em círculo ou quadrado perfeito.").size(12.0));
                    } else if self.current_tool == ToolMode::Smudge {
                        ui.separator();
                        ui.label(egui::RichText::new("👉 Clique e arraste livremente para esfregar e borrar com o pincel.").size(12.0));
                    }

                    if let Some((msg, time)) = &self.status_message {
                        if time.elapsed().as_secs() < 4 {
                            ui.separator();
                            ui.colored_label(Color32::from_rgb(46, 194, 126), msg);
                        }
                    }
                });
            });

        // Central Canvas Area
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_size = ui.available_size();
            let img_w = self.base_image.width() as f32;
            let img_h = self.base_image.height() as f32;

            let scale = (available_size.x / img_w).min(available_size.y / img_h).min(1.0);
            let display_size = Vec2::new(img_w * scale, img_h * scale);

            let (canvas_rect, response) = ui.allocate_exact_size(display_size, egui::Sense::click_and_drag());
            let painter = ui.painter_at(canvas_rect);

            // 1. Draw base screenshot texture
            if let Some(texture) = &self.texture {
                painter.image(
                    texture.id(),
                    canvas_rect,
                    Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }

            // Transform helpers
            let to_screen = |coord: [f32; 2]| -> Pos2 {
                Pos2::new(
                    canvas_rect.min.x + coord[0] * scale,
                    canvas_rect.min.y + coord[1] * scale,
                )
            };
            let to_screen_size = |s: f32| -> f32 { s * scale };

            // 2. Draw existing vector annotations
            for item in &self.scene.items {
                match item {
                    AnnotationItem::Arrow { start, end, color, width } => {
                        let p_start = to_screen(*start);
                        let p_end = to_screen(*end);
                        let col = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                        let stroke = Stroke::new(to_screen_size(*width), col);
                        painter.line_segment([p_start, p_end], stroke);

                        let dir = (p_end - p_start).normalized();
                        let head_len = to_screen_size(width * 4.0).max(12.0);
                        let normal = Vec2::new(-dir.y, dir.x);
                        let h1 = p_end - dir * head_len + normal * (head_len * 0.5);
                        let h2 = p_end - dir * head_len - normal * (head_len * 0.5);
                        painter.line_segment([p_end, h1], stroke);
                        painter.line_segment([p_end, h2], stroke);
                    }
                    AnnotationItem::Rectangle { min, max, color, width, .. } => {
                        let r = Rect::from_min_max(to_screen(*min), to_screen(*max));
                        let col = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                        painter.rect_stroke(r, Rounding::same(3.0), Stroke::new(to_screen_size(*width), col));
                    }
                    AnnotationItem::Ellipse { min, max, color, width, .. } => {
                        let p_min = to_screen(*min);
                        let p_max = to_screen(*max);
                        let cx = (p_min.x + p_max.x) / 2.0;
                        let cy = (p_min.y + p_max.y) / 2.0;
                        let rx = ((p_max.x - p_min.x).abs() / 2.0).max(1.0);
                        let ry = ((p_max.y - p_min.y).abs() / 2.0).max(1.0);
                        let col = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                        let stroke = Stroke::new(to_screen_size(*width), col);

                        // True parametric ellipse
                        let points: Vec<Pos2> = (0..=48)
                            .map(|i| {
                                let theta = (i as f32) * std::f32::consts::TAU / 48.0;
                                Pos2::new(cx + rx * theta.cos(), cy + ry * theta.sin())
                            })
                            .collect();
                        painter.add(egui::Shape::line(points, stroke));
                    }
                    AnnotationItem::StepBadge { pos, number, color, radius } => {
                        let center = to_screen(*pos);
                        let r = to_screen_size(*radius).max(10.0);
                        let col = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                        painter.circle_filled(center, r, col);
                        painter.text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            number.to_string(),
                            egui::FontId::proportional(r * 1.2),
                            Color32::WHITE,
                        );
                    }
                    AnnotationItem::Text { pos, text, color, font_size } => {
                        let p = to_screen(*pos);
                        let col = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                        let rendered_size = (to_screen_size(*font_size)).max(18.0);
                        let font_id = egui::FontId::proportional(rendered_size);

                        // High contrast outline / halo around text
                        let halo_color = if (col.r() as u32 + col.g() as u32 + col.b() as u32) > 380 {
                            Color32::from_black_alpha(220)
                        } else {
                            Color32::from_white_alpha(220)
                        };

                        for (dx, dy) in [(-1.5, 0.0), (1.5, 0.0), (0.0, -1.5), (0.0, 1.5)] {
                            painter.text(
                                Pos2::new(p.x + dx, p.y + dy),
                                egui::Align2::LEFT_TOP,
                                text,
                                font_id.clone(),
                                halo_color,
                            );
                        }
                        painter.text(p, egui::Align2::LEFT_TOP, text, font_id, col);
                    }
                    AnnotationItem::Obfuscate { .. } => {}
                }
            }

            // 3. Handle Single Click placement (StepBadge and Inline Text)
            if response.clicked() {
                if let Some(click_pos) = response.interact_pointer_pos() {
                    if canvas_rect.contains(click_pos) {
                        let img_x = ((click_pos.x - canvas_rect.min.x) / scale).clamp(0.0, img_w);
                        let img_y = ((click_pos.y - canvas_rect.min.y) / scale).clamp(0.0, img_h);

                        if self.current_tool == ToolMode::StepBadge {
                            self.commit_inline_text();
                            self.scene.push_item(AnnotationItem::StepBadge {
                                pos: [img_x, img_y],
                                number: self.scene.next_badge_number,
                                color: self.current_color,
                                radius: (self.stroke_width * 3.5).max(14.0),
                            });
                            self.undo_history.push(UndoEntry::VectorItem);
                            self.redo_history.clear();
                            self.set_status(format!("Passo {} adicionado", self.scene.next_badge_number - 1));
                        } else if self.current_tool == ToolMode::Text {
                            self.commit_inline_text();
                            self.active_inline_text = Some(InlineTextState {
                                pos: [img_x, img_y],
                                text: String::new(),
                            });
                        }
                    }
                }
            }

            // 4. Render Active Inline Text Input directly on the Canvas
            let current_color_32 = self.color32();
            let text_font_size = (to_screen_size(self.stroke_width * 6.0)).max(20.0);
            let mut commit_text = false;

            if let Some(inline) = &mut self.active_inline_text {
                let screen_pos = to_screen(inline.pos);
                let area_rect = Rect::from_min_size(screen_pos, Vec2::new(260.0, 36.0));

                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(area_rect), |ui| {
                    ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_black_alpha(200);
                    ui.visuals_mut().widgets.hovered.bg_fill = Color32::from_black_alpha(230);
                    ui.visuals_mut().widgets.active.bg_fill = Color32::from_black_alpha(240);

                    let edit = egui::TextEdit::singleline(&mut inline.text)
                        .font(egui::FontId::proportional(text_font_size))
                        .text_color(current_color_32)
                        .hint_text("Digite aqui e tecle Enter...");

                    let output = ui.add(edit);
                    output.request_focus();

                    if output.lost_focus() || ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        commit_text = true;
                    }
                });
            }

            if commit_text {
                self.commit_inline_text();
            }

            // 5. Handle Dragging for shapes and freehand Smudge brush
            if !matches!(self.current_tool, ToolMode::StepBadge | ToolMode::Text | ToolMode::Select) {
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.drag_start = Some(pos);
                        self.drag_current = Some(pos);
                        if self.current_tool == ToolMode::Smudge {
                            self.undo_history.push(UndoEntry::RasterSnapshot(self.base_image.clone()));
                            self.redo_history.clear();
                            self.smudge_carried_color = None;
                        }
                    }
                }
                if response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        if self.current_tool == ToolMode::Smudge {
                            if let Some(prev_pos) = self.drag_current {
                                let unscale = |p: Pos2| -> [f32; 2] {
                                    [
                                        (p.x - canvas_rect.min.x) / scale,
                                        (p.y - canvas_rect.min.y) / scale,
                                    ]
                                };
                                let from = unscale(prev_pos);
                                let to = unscale(pos);
                                let brush_radius = (self.stroke_width * 3.0).max(10.0);
                                apply_smudge_step(
                                    &mut self.base_image,
                                    from,
                                    to,
                                    brush_radius,
                                    &mut self.smudge_carried_color,
                                );
                                self.texture = None; // Live update in real time!
                            }
                        }
                        self.drag_current = Some(pos);
                    }
                }
                if response.drag_stopped() {
                    if self.current_tool == ToolMode::Smudge {
                        self.smudge_carried_color = None;
                        self.drag_start = None;
                        self.drag_current = None;
                        self.set_status("Smudge aplicado");
                    } else if let (Some(start), Some(end)) = (self.drag_start.take(), self.drag_current.take()) {
                        let unscale = |p: Pos2| -> Pos2 {
                            Pos2::new(
                                (p.x - canvas_rect.min.x) / scale,
                                (p.y - canvas_rect.min.y) / scale,
                            )
                        };
                        self.finish_current_shape(
                            unscale(start),
                            unscale(end),
                            Rect::from_min_size(Pos2::ZERO, Vec2::new(img_w, img_h)),
                            is_ctrl,
                        );
                    }
                }
            }

            // Draw active preview while dragging
            if let (Some(start), Some(mut curr)) = (self.drag_start, self.drag_current) {
                if is_ctrl && matches!(self.current_tool, ToolMode::Rectangle | ToolMode::Ellipse | ToolMode::Crop) {
                    let dx = curr.x - start.x;
                    let dy = curr.y - start.y;
                    let size = dx.abs().max(dy.abs());
                    curr = Pos2::new(start.x + dx.signum() * size, start.y + dy.signum() * size);
                }

                let stroke = Stroke::new(to_screen_size(self.stroke_width), self.color32());
                match self.current_tool {
                    ToolMode::Arrow => {
                        painter.line_segment([start, curr], stroke);
                    }
                    ToolMode::Rectangle | ToolMode::Crop => {
                        let r = Rect::from_two_pos(start, curr);
                        painter.rect_stroke(r, Rounding::same(3.0), stroke);
                    }
                    ToolMode::Ellipse => {
                        let cx = (start.x + curr.x) / 2.0;
                        let cy = (start.y + curr.y) / 2.0;
                        let rx = ((curr.x - start.x).abs() / 2.0).max(1.0);
                        let ry = ((curr.y - start.y).abs() / 2.0).max(1.0);
                        let points: Vec<Pos2> = (0..=48)
                            .map(|i| {
                                let theta = (i as f32) * std::f32::consts::TAU / 48.0;
                                Pos2::new(cx + rx * theta.cos(), cy + ry * theta.sin())
                            })
                            .collect();
                        painter.add(egui::Shape::line(points, stroke));
                    }
                    ToolMode::Blur | ToolMode::Pixelate => {
                        let r = Rect::from_two_pos(start, curr);
                        painter.rect_filled(r, Rounding::same(3.0), Color32::from_black_alpha(100));
                        painter.rect_stroke(r, Rounding::same(3.0), Stroke::new(1.0_f32, Color32::WHITE));
                    }
                    ToolMode::Smudge => {
                        let brush_radius_screen = to_screen_size((self.stroke_width * 3.0).max(10.0));
                        painter.circle_stroke(curr, brush_radius_screen, Stroke::new(1.5_f32, Color32::from_white_alpha(200)));
                    }
                    _ => {}
                }
            }

            // 6. Live Loupe (Magnifier) & Color Picker under Cursor
            if self.show_loupe {
                if let Some(cursor_pos) = response.hover_pos() {
                    if canvas_rect.contains(cursor_pos) {
                        let pixel_x = ((cursor_pos.x - canvas_rect.min.x) / scale).clamp(0.0, img_w - 1.0) as u32;
                        let pixel_y = ((cursor_pos.y - canvas_rect.min.y) / scale).clamp(0.0, img_h - 1.0) as u32;
                        let px = self.base_image.get_pixel(pixel_x, pixel_y);
                        let hex_code = format!("#{:02X}{:02X}{:02X}", px[0], px[1], px[2]);
                        let rgb_code = format!("rgb({}, {}, {})", px[0], px[1], px[2]);

                        let loupe_size = 92.0;
                        let loupe_pos = Pos2::new(
                            (cursor_pos.x + 20.0).min(canvas_rect.max.x - loupe_size - 10.0),
                            (cursor_pos.y + 20.0).min(canvas_rect.max.y - loupe_size - 40.0),
                        );
                        let loupe_rect = Rect::from_min_size(loupe_pos, Vec2::splat(loupe_size));

                        painter.rect_filled(loupe_rect, Rounding::same(10.0), Color32::from_black_alpha(230));
                        painter.rect_stroke(loupe_rect, Rounding::same(10.0), Stroke::new(1.5_f32, Color32::WHITE));

                        let pixel_color = Color32::from_rgb(px[0], px[1], px[2]);
                        painter.circle_filled(loupe_rect.center(), 12.0, pixel_color);
                        painter.circle_stroke(loupe_rect.center(), 12.0, Stroke::new(1.0_f32, Color32::WHITE));

                        let text_pos = Pos2::new(loupe_rect.min.x + 6.0, loupe_rect.max.y + 4.0);
                        painter.text(
                            text_pos,
                            egui::Align2::LEFT_TOP,
                            format!("{}\n{}", hex_code, rgb_code),
                            egui::FontId::monospace(10.0),
                            Color32::WHITE,
                        );
                    }
                }
            }
        });
    }
}

pub fn run_editor(image: RgbaImage, pin_sender: Option<Sender<RgbaImage>>) -> Result<(), eframe::Error> {
    let width = (image.width() as f32).clamp(700.0, 1600.0);
    let height = (image.height() as f32).clamp(500.0, 1000.0) + 100.0;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Enhanced Shot")
            .with_inner_size([width, height])
            .with_min_inner_size([600.0, 450.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Enhanced Shot",
        options,
        Box::new(|_cc| Ok(Box::new(EditorApp::new(image, pin_sender)))),
    )
}
