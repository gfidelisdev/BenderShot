use crate::export::Exporter;
use crate::model::{AnnotationItem, FontFamily, ObfuscateKind, Scene, TextBackground, ToolMode};
use crate::rasterize::{apply_obfuscation, apply_smudge_step, rasterize_scene};
use crate::ui::theme::{apply_adwaita_theme, AdwaitaThemeMode};
use eframe::egui::{self, Color32, Frame, Margin, Pos2, Rect, Rounding, Stroke, Vec2};
use image::RgbaImage;
use std::sync::mpsc::Sender;

#[derive(Clone)]
struct EditorStateSnapshot {
    base_image: RgbaImage,
    items: Vec<AnnotationItem>,
    next_badge_number: u32,
    selected_item_index: Option<usize>,
}

struct InlineTextState {
    pos: [f32; 2],
    text: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ArrowHandle {
    Start,
    End,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CropHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Inside,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DragMode {
    None,
    TranslateItem,
    ResizeArrow { item_index: usize, handle: ArrowHandle },
    AdjustCrop { handle: CropHandle },
}

pub struct EditorApp {
    base_image: RgbaImage,
    undo_stack: Vec<EditorStateSnapshot>,
    redo_stack: Vec<EditorStateSnapshot>,
    texture: Option<egui::TextureHandle>,
    scene: Scene,
    current_tool: ToolMode,
    current_color: [u8; 4],
    stroke_width: f32,
    drag_start: Option<Pos2>,
    drag_current: Option<Pos2>,
    drag_mode: DragMode,
    crop_selection: Option<Rect>,
    active_inline_text: Option<InlineTextState>,
    show_loupe: bool,
    show_layers_panel: bool,
    selected_item_index: Option<usize>,
    theme_mode: AdwaitaThemeMode,
    selected_font_family: FontFamily,
    selected_text_bg: TextBackground,
    smudge_carried_color: Option<[f32; 4]>,
    status_message: Option<(String, std::time::Instant)>,
    pin_sender: Option<Sender<RgbaImage>>,
}

impl EditorApp {
    pub fn new(base_image: RgbaImage, pin_sender: Option<Sender<RgbaImage>>) -> Self {
        Self {
            base_image,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            texture: None,
            scene: Scene::new(),
            current_tool: ToolMode::Arrow,
            current_color: [224, 27, 36, 255], // Adwaita Red
            stroke_width: 4.0,
            drag_start: None,
            drag_current: None,
            drag_mode: DragMode::None,
            crop_selection: None,
            active_inline_text: None,
            show_loupe: true,
            show_layers_panel: true,
            selected_item_index: None,
            theme_mode: AdwaitaThemeMode::Dark,
            selected_font_family: FontFamily::Sans,
            selected_text_bg: TextBackground::Halo,
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

    fn save_snapshot_for_undo(&mut self) {
        self.undo_stack.push(EditorStateSnapshot {
            base_image: self.base_image.clone(),
            items: self.scene.items.clone(),
            next_badge_number: self.scene.next_badge_number,
            selected_item_index: self.selected_item_index,
        });
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(EditorStateSnapshot {
                base_image: self.base_image.clone(),
                items: self.scene.items.clone(),
                next_badge_number: self.scene.next_badge_number,
                selected_item_index: self.selected_item_index,
            });
            self.base_image = prev.base_image;
            self.scene.items = prev.items;
            self.scene.next_badge_number = prev.next_badge_number;
            self.selected_item_index = prev.selected_item_index;
            self.texture = None;
            self.set_status("Desfeito");
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(EditorStateSnapshot {
                base_image: self.base_image.clone(),
                items: self.scene.items.clone(),
                next_badge_number: self.scene.next_badge_number,
                selected_item_index: self.selected_item_index,
            });
            self.base_image = next.base_image;
            self.scene.items = next.items;
            self.scene.next_badge_number = next.next_badge_number;
            self.selected_item_index = next.selected_item_index;
            self.texture = None;
            self.set_status("Refeito");
        }
    }

    fn commit_inline_text(&mut self) {
        if let Some(inline) = self.active_inline_text.take() {
            let trimmed = inline.text.trim();
            if !trimmed.is_empty() {
                self.save_snapshot_for_undo();
                self.scene.push_item(AnnotationItem::Text {
                    pos: inline.pos,
                    text: trimmed.to_string(),
                    color: self.current_color,
                    font_size: (self.stroke_width * 6.0).max(24.0),
                    family: self.selected_font_family,
                    bg_style: self.selected_text_bg,
                });
                self.selected_item_index = Some(self.scene.items.len() - 1);
                self.set_status("Texto inserido");
            }
        }
    }

    fn finish_current_shape(&mut self, start: Pos2, mut end: Pos2, image_rect: Rect, is_ctrl_constrained: bool) -> bool {
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
                    self.save_snapshot_for_undo();
                    self.scene.push_item(AnnotationItem::Arrow {
                        start: p1,
                        end: p2,
                        color: self.current_color,
                        width: self.stroke_width,
                    });
                    self.selected_item_index = Some(self.scene.items.len() - 1);
                }
            }
            ToolMode::Rectangle => {
                let min = [p1[0].min(p2[0]), p1[1].min(p2[1])];
                let max = [p1[0].max(p2[0]), p1[1].max(p2[1])];
                if (max[0] - min[0]) > 4.0 && (max[1] - min[1]) > 4.0 {
                    self.save_snapshot_for_undo();
                    self.scene.push_item(AnnotationItem::Rectangle {
                        min,
                        max,
                        color: self.current_color,
                        width: self.stroke_width,
                        fill: None,
                    });
                    self.selected_item_index = Some(self.scene.items.len() - 1);
                }
            }
            ToolMode::Ellipse => {
                let min = [p1[0].min(p2[0]), p1[1].min(p2[1])];
                let max = [p1[0].max(p2[0]), p1[1].max(p2[1])];
                if (max[0] - min[0]) > 4.0 && (max[1] - min[1]) > 4.0 {
                    self.save_snapshot_for_undo();
                    self.scene.push_item(AnnotationItem::Ellipse {
                        min,
                        max,
                        color: self.current_color,
                        width: self.stroke_width,
                        fill: None,
                    });
                    self.selected_item_index = Some(self.scene.items.len() - 1);
                }
            }
            ToolMode::Blur => {
                let min = [p1[0].min(p2[0]), p1[1].min(p2[1])];
                let max = [p1[0].max(p2[0]), p1[1].max(p2[1])];
                if (max[0] - min[0]) > 2.0 && (max[1] - min[1]) > 2.0 {
                    self.save_snapshot_for_undo();
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
                    self.save_snapshot_for_undo();
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
                let min_x = p1[0].min(p2[0]).max(0.0);
                let min_y = p1[1].min(p2[1]).max(0.0);
                let max_x = p1[0].max(p2[0]).min(self.base_image.width() as f32);
                let max_y = p1[1].max(p2[1]).min(self.base_image.height() as f32);

                if (max_x - min_x) > 8.0 && (max_y - min_y) > 8.0 {
                    self.crop_selection = Some(Rect::from_min_max(Pos2::new(min_x, min_y), Pos2::new(max_x, max_y)));
                    self.set_status("Trecho selecionado. Ajuste ou escolha Recortar / Copiar Trecho.");
                }
            }
            _ => {}
        }
        false
    }

    fn apply_crop(&mut self) -> bool {
        if let Some(crop_rect) = self.crop_selection.take() {
            let min_x = (crop_rect.min.x.max(0.0) as u32).min(self.base_image.width());
            let min_y = (crop_rect.min.y.max(0.0) as u32).min(self.base_image.height());
            let max_x = (crop_rect.max.x.max(0.0) as u32).min(self.base_image.width());
            let max_y = (crop_rect.max.y.max(0.0) as u32).min(self.base_image.height());

            if max_x > min_x + 8 && max_y > min_y + 8 {
                self.save_snapshot_for_undo();
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
                self.selected_item_index = None;
                self.current_tool = ToolMode::Select;
                self.set_status("Imagem recortada com sucesso");
                return true;
            }
        }
        false
    }

    fn copy_crop_selection(&mut self) {
        if let Some(crop_rect) = self.crop_selection {
            let min_x = (crop_rect.min.x.max(0.0) as u32).min(self.base_image.width());
            let min_y = (crop_rect.min.y.max(0.0) as u32).min(self.base_image.height());
            let max_x = (crop_rect.max.x.max(0.0) as u32).min(self.base_image.width());
            let max_y = (crop_rect.max.y.max(0.0) as u32).min(self.base_image.height());

            if max_x > min_x + 4 && max_y > min_y + 4 {
                let full_rendered = self.export_current_raster();
                let sub_img = image::imageops::crop_imm(
                    &full_rendered,
                    min_x,
                    min_y,
                    max_x - min_x,
                    max_y - min_y,
                )
                .to_image();
                if let Err(e) = Exporter::copy_to_clipboard(&sub_img) {
                    self.set_status(format!("Erro ao copiar trecho: {e}"));
                } else {
                    self.set_status("Trecho copiado para a área de transferência!");
                }
            }
        }
    }

    fn open_image_dialog(&mut self) {
        // Try invoking zenity for a native Wayland / GTK file chooser dialog
        let output = std::process::Command::new("zenity")
            .arg("--file-selection")
            .arg("--title=Abrir Imagem para Edição")
            .arg("--file-filter=Imagens (*.png *.jpg *.jpeg *.webp) | *.png *.jpg *.jpeg *.webp *.PNG *.JPG *.JPEG *.WEBP")
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let path_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let path = std::path::Path::new(&path_str);
                    if let Ok(loaded) = image::open(path) {
                        self.save_snapshot_for_undo();
                        self.base_image = loaded.to_rgba8();
                        self.texture = None;
                        self.scene.clear();
                        self.crop_selection = None;
                        self.selected_item_index = None;
                        self.set_status(format!("Imagem carregada: {}", path.file_name().unwrap_or_default().to_string_lossy()));
                    } else {
                        self.set_status("Falha ao abrir imagem selecionada.");
                    }
                }
            }
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
        if ctx.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
            if self.active_inline_text.is_none() {
                if let Some(idx) = self.selected_item_index {
                    if idx < self.scene.items.len() {
                        self.save_snapshot_for_undo();
                        self.scene.items.remove(idx);
                        self.selected_item_index = None;
                        self.scene.recalc_badge_number();
                        self.set_status("Objeto removido");
                    }
                }
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.active_inline_text.is_some() {
                self.active_inline_text = None;
            } else if self.selected_item_index.is_some() {
                self.selected_item_index = None;
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        let is_ctrl = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);

        // Header Background
        let header_bg = match self.theme_mode {
            AdwaitaThemeMode::Dark => Color32::from_rgb(46, 46, 46),
            AdwaitaThemeMode::Light => Color32::from_rgb(255, 255, 255),
        };

        // Top HeaderBar (GNOME Libadwaita Style)
        egui::TopBottomPanel::top("adwaita_headerbar")
            .frame(Frame::none().fill(header_bg).inner_margin(Margin::symmetric(14.0, 8.0)))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    // App Title
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

                        tool_btn(ToolMode::Select, "↖ Sel");
                        tool_btn(ToolMode::Arrow, "↗ Seta");
                        tool_btn(ToolMode::Rectangle, "▢ Box");
                        tool_btn(ToolMode::Ellipse, "◯ Elipse");
                        tool_btn(ToolMode::Text, "T Texto");
                        tool_btn(ToolMode::StepBadge, &format!("① Passo ({})", next_badge));
                        tool_btn(ToolMode::Blur, "░ Blur");
                        tool_btn(ToolMode::Pixelate, "▦ Pixel");
                        tool_btn(ToolMode::Crop, "⯏ Trecho");
                    });

                    // Contextual buttons when a Crop/Trecho region is selected
                    if let Some(crop_rect) = self.crop_selection {
                        ui.separator();
                        ui.group(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                            let cw = (crop_rect.width()) as u32;
                            let ch = (crop_rect.height()) as u32;
                            ui.label(format!("Trecho: {}x{}px", cw, ch));

                            if ui.button("✂ Cortar Imagem").on_hover_text("Recortar a imagem inteira para o trecho selecionado").clicked() {
                                self.apply_crop();
                            }
                            if ui.button("📋 Copiar Trecho").on_hover_text("Copiar apenas este trecho para a área de transferência sem alterar a imagem original").clicked() {
                                self.copy_crop_selection();
                            }
                            if ui.small_button("✕").on_hover_text("Cancelar seleção de trecho").clicked() {
                                self.crop_selection = None;
                            }
                        });
                    }

                    ui.separator();

                    // Text-specific controls if Text tool is active or a text item is selected
                    let is_text_focused = self.current_tool == ToolMode::Text
                        || self.selected_item_index.and_then(|idx| self.scene.items.get(idx)).map_or(false, |it| matches!(it, AnnotationItem::Text { .. }));

                    if is_text_focused {
                        ui.group(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                            ui.label("Fonte:");
                            if ui.selectable_label(self.selected_font_family == FontFamily::Sans, "Sans").clicked() {
                                self.selected_font_family = FontFamily::Sans;
                                if let Some(idx) = self.selected_item_index {
                                    if let Some(AnnotationItem::Text { family, .. }) = self.scene.items.get_mut(idx) {
                                        *family = FontFamily::Sans;
                                    }
                                }
                            }
                            if ui.selectable_label(self.selected_font_family == FontFamily::Monospace, "Mono").clicked() {
                                self.selected_font_family = FontFamily::Monospace;
                                if let Some(idx) = self.selected_item_index {
                                    if let Some(AnnotationItem::Text { family, .. }) = self.scene.items.get_mut(idx) {
                                        *family = FontFamily::Monospace;
                                    }
                                }
                            }
                            if ui.selectable_label(self.selected_font_family == FontFamily::Serif, "Serif").clicked() {
                                self.selected_font_family = FontFamily::Serif;
                                if let Some(idx) = self.selected_item_index {
                                    if let Some(AnnotationItem::Text { family, .. }) = self.scene.items.get_mut(idx) {
                                        *family = FontFamily::Serif;
                                    }
                                }
                            }

                            ui.separator();
                            ui.label("Fundo:");
                            if ui.selectable_label(self.selected_text_bg == TextBackground::Halo, "Halo").clicked() {
                                self.selected_text_bg = TextBackground::Halo;
                                if let Some(idx) = self.selected_item_index {
                                    if let Some(AnnotationItem::Text { bg_style, .. }) = self.scene.items.get_mut(idx) {
                                        *bg_style = TextBackground::Halo;
                                    }
                                }
                            }
                            if ui.selectable_label(self.selected_text_bg == TextBackground::FilledTag, "Tag").clicked() {
                                self.selected_text_bg = TextBackground::FilledTag;
                                if let Some(idx) = self.selected_item_index {
                                    if let Some(AnnotationItem::Text { bg_style, .. }) = self.scene.items.get_mut(idx) {
                                        *bg_style = TextBackground::FilledTag;
                                    }
                                }
                            }
                        });
                        ui.separator();
                    }

                    // Stroke width
                    ui.label("Traço:");
                    let prev_width = self.stroke_width;
                    ui.add(egui::Slider::new(&mut self.stroke_width, 1.0..=20.0).show_value(false));
                    if (prev_width - self.stroke_width).abs() > 0.05 {
                        if let Some(idx) = self.selected_item_index {
                            if let Some(item) = self.scene.items.get_mut(idx) {
                                item.set_width(self.stroke_width);
                            }
                        }
                    }

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
                            if let Some(idx) = self.selected_item_index {
                                if let Some(item) = self.scene.items.get_mut(idx) {
                                    item.set_color(color);
                                }
                            }
                        }
                    }

                    ui.separator();

                    // Undo / Redo
                    if ui.button("↶ Desfazer").on_hover_text("Desfazer (Ctrl+Z)").clicked() {
                        self.undo();
                    }
                    if ui.button("↷ Refazer").on_hover_text("Refazer (Ctrl+Shift+Z)").clicked() {
                        self.redo();
                    }

                    ui.separator();

                    // Action Buttons
                    let copy_btn = egui::Button::new(
                        egui::RichText::new("📋 Copiar")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(53, 132, 228)) // Adwaita Blue
                    .rounding(Rounding::same(8.0));

                    if ui.add(copy_btn).clicked() {
                        self.copy_and_exit(ctx);
                    }

                    if ui.button("📂 Abrir").on_hover_text("Abrir imagem do disco para edição").clicked() {
                        self.open_image_dialog();
                    }

                    if ui.button("💾 Salvar").clicked() {
                        self.commit_inline_text();
                        let final_img = self.export_current_raster();
                        if let Ok(saved_path) = Exporter::auto_save(&final_img) {
                            self.set_status(format!("Salvo em: {:?}", saved_path));
                        }
                    }

                    if ui.button("📌 Fixar").on_hover_text("Manter screenshot flutuando no topo").clicked() {
                        self.commit_inline_text();
                        let final_img = self.export_current_raster();
                        if let Some(ref sender) = self.pin_sender {
                            let _ = sender.send(final_img);
                        }
                        self.set_status("Screenshot fixado na tela");
                    }

                    ui.separator();

                    // Toggle Layers Panel button
                    let layers_btn_text = if self.show_layers_panel { "☰ Camadas" } else { "☷ Camadas" };
                    if ui.selectable_label(self.show_layers_panel, layers_btn_text).clicked() {
                        self.show_layers_panel = !self.show_layers_panel;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Dark / Light Theme Toggle
                        let theme_icon = match self.theme_mode {
                            AdwaitaThemeMode::Dark => "☀️",
                            AdwaitaThemeMode::Light => "🌙",
                        };
                        if ui.button(theme_icon).on_hover_text("Alternar tema Claro/Escuro").clicked() {
                            self.theme_mode = match self.theme_mode {
                                AdwaitaThemeMode::Dark => AdwaitaThemeMode::Light,
                                AdwaitaThemeMode::Light => AdwaitaThemeMode::Dark,
                            };
                        }
                    });
                });
            });

        // Right-hand Layers Panel (Photoshop / Figma style)
        if self.show_layers_panel {
            egui::SidePanel::right("layers_panel")
                .frame(Frame::none().fill(header_bg).inner_margin(Margin::same(10.0)))
                .resizable(false)
                .default_width(220.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("📑 Camadas").strong().size(13.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🗑 Limpar").on_hover_text("Remover todas as anotações").clicked() {
                                if !self.scene.items.is_empty() {
                                    self.save_snapshot_for_undo();
                                    self.scene.clear();
                                    self.selected_item_index = None;
                                    self.set_status("Camadas limpas");
                                }
                            }
                        });
                    });
                    ui.separator();

                    if self.scene.items.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label(egui::RichText::new("Nenhuma anotação ainda.\nDesenhe na imagem!").weak());
                        });
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let total_items = self.scene.items.len();
                            // Render from top (highest z-index) to bottom
                            for display_idx in 0..total_items {
                                let real_idx = total_items - 1 - display_idx;
                                let is_selected = self.selected_item_index == Some(real_idx);

                                let item = &self.scene.items[real_idx];
                                let label = item.layer_name();
                                let item_col = item.color();
                                let dot_color = Color32::from_rgba_unmultiplied(item_col[0], item_col[1], item_col[2], item_col[3]);

                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        // Color dot
                                        let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                                        ui.painter().circle_filled(dot_rect.center(), 4.5, dot_color);

                                        // Selectable row
                                        if ui.selectable_label(is_selected, &label).clicked() {
                                            self.selected_item_index = Some(real_idx);
                                            self.current_tool = ToolMode::Select;
                                        }

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            // Delete layer button
                                            if ui.small_button("✕").on_hover_text("Excluir camada").clicked() {
                                                self.save_snapshot_for_undo();
                                                self.scene.items.remove(real_idx);
                                                self.selected_item_index = None;
                                                self.scene.recalc_badge_number();
                                            }

                                            // Reorder up (closer to front)
                                            if real_idx + 1 < total_items {
                                                if ui.small_button("▲").on_hover_text("Mover para frente").clicked() {
                                                    self.save_snapshot_for_undo();
                                                    self.scene.items.swap(real_idx, real_idx + 1);
                                                    self.selected_item_index = Some(real_idx + 1);
                                                }
                                            }

                                            // Reorder down (closer to back)
                                            if real_idx > 0 {
                                                if ui.small_button("▼").on_hover_text("Mover para trás").clicked() {
                                                    self.save_snapshot_for_undo();
                                                    self.scene.items.swap(real_idx, real_idx - 1);
                                                    self.selected_item_index = Some(real_idx - 1);
                                                }
                                            }
                                        });
                                    });
                                });
                            }
                        });
                    }
                });
        }

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
                        ui.label(egui::RichText::new("👉 Clique na imagem para posicionar a caixa de texto.").size(12.0));
                    } else if self.current_tool == ToolMode::StepBadge {
                        ui.separator();
                        ui.label(egui::RichText::new("👉 Clique diretamente para estampar o próximo passo.").size(12.0));
                    } else if self.current_tool == ToolMode::Smudge {
                        ui.separator();
                        ui.label(egui::RichText::new("👉 Clique e arraste livremente para esfregar e borrar a área.").size(12.0));
                    } else if self.current_tool == ToolMode::Select {
                        ui.separator();
                        ui.label(egui::RichText::new("👉 Clique em um objeto para selecionar, arrastar ou tecle Delete para remover.").size(12.0));
                    } else if matches!(self.current_tool, ToolMode::Ellipse | ToolMode::Rectangle) {
                        ui.separator();
                        ui.label(egui::RichText::new("💡 Segure Ctrl para travar em círculo ou quadrado perfeito.").size(12.0));
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

            if img_w <= 0.0 || img_h <= 0.0 {
                return;
            }

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
            for (idx, item) in self.scene.items.iter().enumerate() {
                let is_selected = self.selected_item_index == Some(idx);

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
                    AnnotationItem::Text { pos, text, color, font_size, family, bg_style } => {
                        let p = to_screen(*pos);
                        let col = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                        let rendered_size = (to_screen_size(*font_size)).max(18.0);
                        let font_id = match family {
                            FontFamily::Sans => egui::FontId::proportional(rendered_size),
                            FontFamily::Monospace => egui::FontId::monospace(rendered_size),
                            FontFamily::Serif => egui::FontId::proportional(rendered_size),
                        };

                        if *bg_style == TextBackground::FilledTag {
                            let text_len = text.len() as f32 * rendered_size * 0.65;
                            let tag_rect = Rect::from_min_size(
                                Pos2::new(p.x - 6.0, p.y - 3.0),
                                Vec2::new(text_len + 12.0, rendered_size + 6.0),
                            );
                            painter.rect_filled(tag_rect, Rounding::same(4.0), Color32::from_black_alpha(200));
                        } else {
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
                        }
                        painter.text(p, egui::Align2::LEFT_TOP, text, font_id, col);
                    }
                    AnnotationItem::Obfuscate { .. } => {}
                }

                // If layer is selected, draw selection indicators
                if is_selected {
                    match item {
                        AnnotationItem::Arrow { start, end, .. } => {
                            let p_start = to_screen(*start);
                            let p_end = to_screen(*end);
                            let handle_r = 7.0_f32;
                            let handle_fill = Color32::from_rgb(53, 132, 228);
                            let handle_stroke = Stroke::new(2.0_f32, Color32::WHITE);

                            // Start point handle (circle with white border)
                            painter.circle_filled(p_start, handle_r, handle_fill);
                            painter.circle_stroke(p_start, handle_r, handle_stroke);

                            // End point handle (circle with white border)
                            painter.circle_filled(p_end, handle_r, handle_fill);
                            painter.circle_stroke(p_end, handle_r, handle_stroke);
                        }
                        _ => {
                            let (b_min, b_max) = item.bounding_box();
                            let s_min = to_screen(b_min);
                            let s_max = to_screen(b_max);
                            let sel_rect = Rect::from_min_max(
                                Pos2::new(s_min.x - 4.0, s_min.y - 4.0),
                                Pos2::new(s_max.x + 4.0, s_max.y + 4.0),
                            );
                            painter.rect_stroke(sel_rect, Rounding::same(3.0), Stroke::new(1.5_f32, Color32::from_rgb(53, 132, 228)));
                        }
                    }
                }
            }

            // 2.5 Draw Crop Selection Overlay and Handles if active
            if let Some(crop_rect) = self.crop_selection {
                let s_min = to_screen([crop_rect.min.x, crop_rect.min.y]);
                let s_max = to_screen([crop_rect.max.x, crop_rect.max.y]);
                let screen_crop_rect = Rect::from_min_max(s_min, s_max);

                // Darken surrounding regions outside crop
                let top_dim = Rect::from_min_max(canvas_rect.min, Pos2::new(canvas_rect.max.x, screen_crop_rect.min.y));
                let bottom_dim = Rect::from_min_max(Pos2::new(canvas_rect.min.x, screen_crop_rect.max.y), canvas_rect.max);
                let left_dim = Rect::from_min_max(Pos2::new(canvas_rect.min.x, screen_crop_rect.min.y), Pos2::new(screen_crop_rect.min.x, screen_crop_rect.max.y));
                let right_dim = Rect::from_min_max(Pos2::new(screen_crop_rect.max.x, screen_crop_rect.min.y), Pos2::new(canvas_rect.max.x, screen_crop_rect.max.y));

                for dim in [top_dim, bottom_dim, left_dim, right_dim] {
                    if dim.is_positive() {
                        painter.rect_filled(dim, Rounding::ZERO, Color32::from_black_alpha(140));
                    }
                }

                // Stroke crop boundary
                painter.rect_stroke(screen_crop_rect, Rounding::ZERO, Stroke::new(2.0_f32, Color32::from_rgb(53, 132, 228)));

                // Draw Corner Handles
                let corner_handle_r = 6.0_f32;
                let handle_fill = Color32::from_rgb(53, 132, 228);
                let handle_stroke = Stroke::new(2.0_f32, Color32::WHITE);
                for pt in [
                    screen_crop_rect.left_top(),
                    screen_crop_rect.right_top(),
                    screen_crop_rect.left_bottom(),
                    screen_crop_rect.right_bottom(),
                ] {
                    painter.circle_filled(pt, corner_handle_r, handle_fill);
                    painter.circle_stroke(pt, corner_handle_r, handle_stroke);
                }
            }

            // 3. Handle Single Click placement (StepBadge, Inline Text, or Selection)
            if response.clicked() {
                if let Some(click_pos) = response.interact_pointer_pos() {
                    if canvas_rect.contains(click_pos) {
                        let img_x = ((click_pos.x - canvas_rect.min.x) / scale).clamp(0.0, img_w);
                        let img_y = ((click_pos.y - canvas_rect.min.y) / scale).clamp(0.0, img_h);

                        if self.current_tool == ToolMode::StepBadge {
                            self.commit_inline_text();
                            self.save_snapshot_for_undo();
                            self.scene.push_item(AnnotationItem::StepBadge {
                                pos: [img_x, img_y],
                                number: self.scene.next_badge_number,
                                color: self.current_color,
                                radius: (self.stroke_width * 3.5).max(14.0),
                            });
                            self.selected_item_index = Some(self.scene.items.len() - 1);
                            self.set_status(format!("Passo {} adicionado", self.scene.next_badge_number - 1));
                        } else if self.current_tool == ToolMode::Text {
                            self.commit_inline_text();
                            self.active_inline_text = Some(InlineTextState {
                                pos: [img_x, img_y],
                                text: String::new(),
                            });
                        } else if self.current_tool == ToolMode::Select {
                            // First check if an arrow handle of the currently selected arrow was clicked
                            let mut clicked_handle = false;
                            if let Some(sel_idx) = self.selected_item_index {
                                if let Some(AnnotationItem::Arrow { start, end, .. }) = self.scene.items.get(sel_idx) {
                                    let p_start = to_screen(*start);
                                    let p_end = to_screen(*end);
                                    let handle_click_dist = 14.0_f32;
                                    if click_pos.distance(p_start) <= handle_click_dist || click_pos.distance(p_end) <= handle_click_dist {
                                        clicked_handle = true;
                                    }
                                }
                            }

                            if !clicked_handle {
                                // Hit test items in reverse order
                                let mut found = None;
                                for (idx, item) in self.scene.items.iter().enumerate().rev() {
                                    let (b_min, b_max) = item.bounding_box();
                                    let pad = 8.0_f32;
                                    if img_x >= b_min[0] - pad && img_x <= b_max[0] + pad
                                        && img_y >= b_min[1] - pad && img_y <= b_max[1] + pad {
                                        found = Some(idx);
                                        break;
                                    }
                                }
                                self.selected_item_index = found;
                            }
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

            // 5. Handle Dragging (Shapes, Moving Selected Items, Resizing Arrows, Adjusting Crop, and Smudge Brush)
            let mut cropped_now = false;

            if !matches!(self.current_tool, ToolMode::StepBadge | ToolMode::Text) {
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.drag_start = Some(pos);
                        self.drag_current = Some(pos);
                        self.drag_mode = DragMode::None;

                        if self.current_tool == ToolMode::Smudge {
                            self.save_snapshot_for_undo();
                            self.smudge_carried_color = None;
                        } else if self.current_tool == ToolMode::Crop && self.crop_selection.is_some() {
                            let crop = self.crop_selection.unwrap();
                            let s_min = to_screen([crop.min.x, crop.min.y]);
                            let s_max = to_screen([crop.max.x, crop.max.y]);
                            let screen_rect = Rect::from_min_max(s_min, s_max);
                            let grab_d = 14.0_f32;

                            if pos.distance(screen_rect.left_top()) <= grab_d {
                                self.drag_mode = DragMode::AdjustCrop { handle: CropHandle::TopLeft };
                            } else if pos.distance(screen_rect.right_top()) <= grab_d {
                                self.drag_mode = DragMode::AdjustCrop { handle: CropHandle::TopRight };
                            } else if pos.distance(screen_rect.left_bottom()) <= grab_d {
                                self.drag_mode = DragMode::AdjustCrop { handle: CropHandle::BottomLeft };
                            } else if pos.distance(screen_rect.right_bottom()) <= grab_d {
                                self.drag_mode = DragMode::AdjustCrop { handle: CropHandle::BottomRight };
                            } else if screen_rect.contains(pos) {
                                self.drag_mode = DragMode::AdjustCrop { handle: CropHandle::Inside };
                            }
                        } else if self.current_tool == ToolMode::Select {
                            let mut handle_drag = None;
                            if let Some(sel_idx) = self.selected_item_index {
                                if let Some(AnnotationItem::Arrow { start, end, .. }) = self.scene.items.get(sel_idx) {
                                    let p_start = to_screen(*start);
                                    let p_end = to_screen(*end);
                                    let grab_dist = 14.0_f32;
                                    if pos.distance(p_start) <= grab_dist {
                                        handle_drag = Some((sel_idx, ArrowHandle::Start));
                                    } else if pos.distance(p_end) <= grab_dist {
                                        handle_drag = Some((sel_idx, ArrowHandle::End));
                                    }
                                }
                            }

                            if let Some((idx, handle)) = handle_drag {
                                self.save_snapshot_for_undo();
                                self.drag_mode = DragMode::ResizeArrow {
                                    item_index: idx,
                                    handle,
                                };
                            } else if self.selected_item_index.is_some() {
                                self.save_snapshot_for_undo();
                                self.drag_mode = DragMode::TranslateItem;
                            }
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
                                self.texture = None;
                            }
                        } else if self.current_tool == ToolMode::Crop {
                            if let (DragMode::AdjustCrop { handle }, Some(prev_pos)) = (self.drag_mode, self.drag_current) {
                                if let Some(mut crop) = self.crop_selection {
                                    let delta_x = (pos.x - prev_pos.x) / scale;
                                    let delta_y = (pos.y - prev_pos.y) / scale;
                                    let img_pos_x = ((pos.x - canvas_rect.min.x) / scale).clamp(0.0, img_w);
                                    let img_pos_y = ((pos.y - canvas_rect.min.y) / scale).clamp(0.0, img_h);

                                    match handle {
                                        CropHandle::TopLeft => {
                                            crop.min.x = img_pos_x.min(crop.max.x - 8.0);
                                            crop.min.y = img_pos_y.min(crop.max.y - 8.0);
                                        }
                                        CropHandle::TopRight => {
                                            crop.max.x = img_pos_x.max(crop.min.x + 8.0);
                                            crop.min.y = img_pos_y.min(crop.max.y - 8.0);
                                        }
                                        CropHandle::BottomLeft => {
                                            crop.min.x = img_pos_x.min(crop.max.x - 8.0);
                                            crop.max.y = img_pos_y.max(crop.min.y + 8.0);
                                        }
                                        CropHandle::BottomRight => {
                                            crop.max.x = img_pos_x.max(crop.min.x + 8.0);
                                            crop.max.y = img_pos_y.max(crop.min.y + 8.0);
                                        }
                                        CropHandle::Inside => {
                                            let w = crop.width();
                                            let h = crop.height();
                                            let new_min_x = (crop.min.x + delta_x).clamp(0.0, img_w - w);
                                            let new_min_y = (crop.min.y + delta_y).clamp(0.0, img_h - h);
                                            crop = Rect::from_min_size(Pos2::new(new_min_x, new_min_y), Vec2::new(w, h));
                                        }
                                    }
                                    self.crop_selection = Some(crop);
                                }
                            }
                        } else if self.current_tool == ToolMode::Select {
                            match self.drag_mode {
                                DragMode::ResizeArrow { item_index, handle } => {
                                    let img_pos = [
                                        ((pos.x - canvas_rect.min.x) / scale).clamp(0.0, img_w),
                                        ((pos.y - canvas_rect.min.y) / scale).clamp(0.0, img_h),
                                    ];
                                    if let Some(AnnotationItem::Arrow { start, end, .. }) = self.scene.items.get_mut(item_index) {
                                        match handle {
                                            ArrowHandle::Start => *start = img_pos,
                                            ArrowHandle::End => *end = img_pos,
                                        }
                                    }
                                }
                                DragMode::TranslateItem => {
                                    if let (Some(prev_pos), Some(sel_idx)) = (self.drag_current, self.selected_item_index) {
                                        if let Some(item) = self.scene.items.get_mut(sel_idx) {
                                            let delta_x = (pos.x - prev_pos.x) / scale;
                                            let delta_y = (pos.y - prev_pos.y) / scale;
                                            item.translate([delta_x, delta_y]);
                                        }
                                    }
                                }
                                _ => {}
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
                        self.drag_mode = DragMode::None;
                        self.set_status("Smudge aplicado");
                    } else if self.current_tool == ToolMode::Select {
                        self.drag_start = None;
                        self.drag_current = None;
                        self.drag_mode = DragMode::None;
                    } else if self.current_tool == ToolMode::Crop && matches!(self.drag_mode, DragMode::AdjustCrop { .. }) {
                        self.drag_start = None;
                        self.drag_current = None;
                        self.drag_mode = DragMode::None;
                    } else if let (Some(start), Some(end)) = (self.drag_start.take(), self.drag_current.take()) {
                        self.drag_mode = DragMode::None;
                        let unscale = |p: Pos2| -> Pos2 {
                            Pos2::new(
                                (p.x - canvas_rect.min.x) / scale,
                                (p.y - canvas_rect.min.y) / scale,
                            )
                        };
                        cropped_now = self.finish_current_shape(
                            unscale(start),
                            unscale(end),
                            Rect::from_min_size(Pos2::ZERO, Vec2::new(img_w, img_h)),
                            is_ctrl,
                        );
                    }
                }
            }

            if cropped_now {
                ctx.request_repaint();
                return;
            }

            // Draw active preview while dragging shapes
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
                        let dir = (curr - start).normalized();
                        let head_len = to_screen_size(self.stroke_width * 4.0).max(12.0);
                        if (curr - start).length() > 4.0 {
                            let normal = Vec2::new(-dir.y, dir.x);
                            let h1 = curr - dir * head_len + normal * (head_len * 0.5);
                            let h2 = curr - dir * head_len - normal * (head_len * 0.5);
                            painter.line_segment([curr, h1], stroke);
                            painter.line_segment([curr, h2], stroke);
                        }
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

            // 6. Live Loupe (Magnifier) & Color Picker with Strict Bounds Checking
            if self.show_loupe {
                if let Some(cursor_pos) = response.hover_pos() {
                    if canvas_rect.contains(cursor_pos) {
                        let w = self.base_image.width();
                        let h = self.base_image.height();
                        if w > 0 && h > 0 {
                            let pixel_x = ((cursor_pos.x - canvas_rect.min.x) / scale).clamp(0.0, (w - 1) as f32) as u32;
                            let pixel_y = ((cursor_pos.y - canvas_rect.min.y) / scale).clamp(0.0, (h - 1) as f32) as u32;

                            if pixel_x < w && pixel_y < h {
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
                }
            }
        });
    }
}

pub fn run_editor(image: RgbaImage, pin_sender: Option<Sender<RgbaImage>>) -> Result<(), eframe::Error> {
    let width = (image.width() as f32).clamp(700.0, 1600.0) + 220.0; // Extra room for layers panel
    let height = (image.height() as f32).clamp(500.0, 1000.0) + 100.0;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Enhanced Shot")
            .with_inner_size([width, height])
            .with_min_inner_size([700.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Enhanced Shot",
        options,
        Box::new(|_cc| Ok(Box::new(EditorApp::new(image, pin_sender)))),
    )
}
