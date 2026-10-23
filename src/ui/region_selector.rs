//! Live-screen region selector (Lightshot style).
//!
//! A borderless, fullscreen, *transparent* overlay is shown on top of the live desktop (no
//! screenshot is taken beforehand). The user drags a rectangle, adjusts it with the anchor
//! handles, and confirms with the attached tick button (or cancels with ✕ / Esc). Only after the
//! overlay closes is the desktop captured and cropped (see `CaptureBackend::capture_region`).

use crate::capture::RegionSelection;
use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, Rounding, Stroke, Vec2};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandleType {
    TopLeft,
    TopCenter,
    TopRight,
    LeftCenter,
    RightCenter,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Inside,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DragMode {
    None,
    Creating { start: Pos2 },
    Moving { offset: Vec2 },
    Resizing(HandleType),
}

const ACCENT: Color32 = Color32::from_rgb(53, 132, 228);
const MIN_SIZE: f32 = 8.0;
const GRAB_DIST: f32 = 12.0;

pub struct RegionSelectorApp {
    selection: Option<Rect>,
    drag_mode: DragMode,
    result: Arc<Mutex<Option<RegionSelection>>>,
    closing: bool,
    focused_once: bool,
}

impl RegionSelectorApp {
    pub fn new(result: Arc<Mutex<Option<RegionSelection>>>) -> Self {
        Self { selection: None, drag_mode: DragMode::None, result, closing: false, focused_once: false }
    }

    fn confirm(&mut self, ctx: &egui::Context, screen: Rect) {
        if let Some(sel) = self.selection {
            if sel.width() >= MIN_SIZE && sel.height() >= MIN_SIZE {
                *self.result.lock().unwrap() = Some(RegionSelection {
                    rect: [sel.min.x - screen.min.x, sel.min.y - screen.min.y, sel.max.x - screen.min.x, sel.max.y - screen.min.y],
                    overlay_size: [screen.width(), screen.height()],
                });
                self.close(ctx);
            }
        }
    }

    fn close(&mut self, ctx: &egui::Context) {
        self.closing = true;
        // Do NOT send ViewportCommand::Visible(false) before Close: on Windows (Win32),
        // hiding the window suppresses WM_PAINT (RedrawRequested), preventing eframe/winit
        // from completing the close cycle and leaving the process stuck at 100% CPU.
        // Returning early in `update()` while `self.closing` is true already clears the
        // transparent framebuffer to [0, 0, 0, 0] immediately.
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        ctx.request_repaint();
    }

    fn handles(sel: Rect) -> [(HandleType, Pos2, CursorIcon); 8] {
        [
            (HandleType::TopLeft, sel.left_top(), CursorIcon::ResizeNorthWest),
            (HandleType::TopCenter, sel.center_top(), CursorIcon::ResizeVertical),
            (HandleType::TopRight, sel.right_top(), CursorIcon::ResizeNorthEast),
            (HandleType::LeftCenter, sel.left_center(), CursorIcon::ResizeHorizontal),
            (HandleType::RightCenter, sel.right_center(), CursorIcon::ResizeHorizontal),
            (HandleType::BottomLeft, sel.left_bottom(), CursorIcon::ResizeNorthEast),
            (HandleType::BottomCenter, sel.center_bottom(), CursorIcon::ResizeVertical),
            (HandleType::BottomRight, sel.right_bottom(), CursorIcon::ResizeNorthWest),
        ]
    }

    /// Action bar (✕ / ✔) attached below the selection's bottom-right corner (flips inside if needed).
    fn action_bar(sel: Rect, screen: Rect) -> (Rect, Rect, Rect) {
        let (bar_w, bar_h, margin) = (88.0_f32, 42.0_f32, 8.0_f32);
        let mut x = sel.max.x - bar_w;
        let mut y = sel.max.y + margin;
        if y + bar_h > screen.max.y {
            y = sel.max.y - bar_h - margin;
        }
        x = x.clamp(screen.min.x + 4.0, (screen.max.x - bar_w - 4.0).max(screen.min.x + 4.0));
        y = y.clamp(screen.min.y + 4.0, (screen.max.y - bar_h - 4.0).max(screen.min.y + 4.0));
        let bar = Rect::from_min_size(Pos2::new(x, y), Vec2::new(bar_w, bar_h));
        let cancel = Rect::from_center_size(Pos2::new(bar.min.x + 22.0, bar.center().y), Vec2::splat(32.0));
        let tick = Rect::from_center_size(Pos2::new(bar.max.x - 22.0, bar.center().y), Vec2::splat(32.0));
        (bar, cancel, tick)
    }
}

impl eframe::App for RegionSelectorApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.closing {
            return;
        }
        if !self.focused_once {
            self.focused_once = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *self.result.lock().unwrap() = None;
            self.close(ctx);
            return;
        }

        let screen = ctx.screen_rect();
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.confirm(ctx, screen);
            if self.closing {
                return;
            }
        }

        let (pointer, pressed, down, released) = ctx.input(|i| {
            (i.pointer.hover_pos(), i.pointer.primary_pressed(), i.pointer.primary_down(), i.pointer.primary_released())
        });

        let bar = self
            .selection
            .filter(|s| s.width() >= MIN_SIZE && s.height() >= MIN_SIZE)
            .map(|s| Self::action_bar(s, screen));
        let over_bar = matches!(self.drag_mode, DragMode::None)
            && pointer.zip(bar).map_or(false, |(p, (b, _, _))| b.contains(p));

        // --- Interaction -------------------------------------------------------------
        if over_bar {
            ctx.set_cursor_icon(CursorIcon::PointingHand);
            if released {
                if let (Some(p), Some((_, cancel, tick))) = (pointer, bar) {
                    if tick.contains(p) {
                        self.confirm(ctx, screen);
                        return;
                    }
                    if cancel.contains(p) {
                        *self.result.lock().unwrap() = None;
                        self.close(ctx);
                        return;
                    }
                }
            }
        } else {
            let hovered = match (self.selection, pointer) {
                (Some(sel), Some(p)) => Self::handles(sel)
                    .into_iter()
                    .find(|(_, hp, _)| p.distance(*hp) <= GRAB_DIST)
                    .map(|(h, _, c)| (h, c))
                    .or_else(|| sel.contains(p).then_some((HandleType::Inside, CursorIcon::Move))),
                _ => None,
            };

            let cursor = match self.drag_mode {
                DragMode::Creating { .. } => CursorIcon::Crosshair,
                DragMode::Moving { .. } => CursorIcon::Grabbing,
                DragMode::Resizing(h) => self
                    .selection
                    .and_then(|s| Self::handles(s).into_iter().find(|(t, _, _)| *t == h).map(|(_, _, c)| c))
                    .unwrap_or(CursorIcon::Crosshair),
                DragMode::None => hovered.map_or(CursorIcon::Crosshair, |(_, c)| c),
            };
            ctx.set_cursor_icon(cursor);

            if pressed {
                if let Some(p) = pointer {
                    self.drag_mode = match hovered {
                        Some((HandleType::Inside, _)) => DragMode::Moving { offset: p - self.selection.unwrap().min },
                        Some((h, _)) => DragMode::Resizing(h),
                        None => {
                            self.selection = Some(Rect::from_two_pos(p, p));
                            DragMode::Creating { start: p }
                        }
                    };
                }
            }

            if down {
                if let (Some(p), Some(mut sel)) = (pointer, self.selection) {
                    let px = p.x.clamp(screen.min.x, screen.max.x);
                    let py = p.y.clamp(screen.min.y, screen.max.y);
                    match self.drag_mode {
                        DragMode::Creating { start } => sel = Rect::from_two_pos(start, Pos2::new(px, py)),
                        DragMode::Moving { offset } => {
                            let size = sel.size();
                            let min = Pos2::new(
                                (p.x - offset.x).clamp(screen.min.x, screen.max.x - size.x),
                                (p.y - offset.y).clamp(screen.min.y, screen.max.y - size.y),
                            );
                            sel = Rect::from_min_size(min, size);
                        }
                        DragMode::Resizing(h) => {
                            use HandleType::*;
                            if matches!(h, TopLeft | LeftCenter | BottomLeft) {
                                sel.min.x = px.min(sel.max.x - MIN_SIZE);
                            }
                            if matches!(h, TopRight | RightCenter | BottomRight) {
                                sel.max.x = px.max(sel.min.x + MIN_SIZE);
                            }
                            if matches!(h, TopLeft | TopCenter | TopRight) {
                                sel.min.y = py.min(sel.max.y - MIN_SIZE);
                            }
                            if matches!(h, BottomLeft | BottomCenter | BottomRight) {
                                sel.max.y = py.max(sel.min.y + MIN_SIZE);
                            }
                        }
                        DragMode::None => {}
                    }
                    self.selection = Some(sel);
                }
            } else if !matches!(self.drag_mode, DragMode::None) {
                self.drag_mode = DragMode::None;
                // A simple click without dragging leaves no usable selection.
                if self.selection.map_or(false, |s| s.width() < MIN_SIZE || s.height() < MIN_SIZE) {
                    self.selection = None;
                }
            }
        }

        // --- Rendering (on a transparent window: the live desktop shows through) -----
        let painter = ctx.layer_painter(egui::LayerId::background());
        let veil = Color32::from_black_alpha(110);

        match self.selection {
            Some(sel) if sel.width() >= 1.0 && sel.height() >= 1.0 => {
                for dim in [
                    Rect::from_min_max(screen.min, Pos2::new(screen.max.x, sel.min.y)),
                    Rect::from_min_max(Pos2::new(screen.min.x, sel.max.y), screen.max),
                    Rect::from_min_max(Pos2::new(screen.min.x, sel.min.y), Pos2::new(sel.min.x, sel.max.y)),
                    Rect::from_min_max(Pos2::new(sel.max.x, sel.min.y), Pos2::new(screen.max.x, sel.max.y)),
                ] {
                    if dim.is_positive() {
                        painter.rect_filled(dim, Rounding::ZERO, veil);
                    }
                }
                painter.rect_stroke(sel, Rounding::ZERO, Stroke::new(2.0_f32, ACCENT));

                // Size badge in physical pixels
                let ppp = ctx.pixels_per_point();
                let text = format!("{} × {} px", (sel.width() * ppp).round() as u32, (sel.height() * ppp).round() as u32);
                let badge_pos = if sel.min.y > screen.min.y + 26.0 {
                    Pos2::new(sel.min.x, sel.min.y - 24.0)
                } else {
                    Pos2::new(sel.min.x + 6.0, sel.min.y + 6.0)
                };
                let badge = Rect::from_min_size(badge_pos, Vec2::new(104.0, 20.0));
                painter.rect_filled(badge, Rounding::same(4.0), Color32::from_black_alpha(200));
                painter.text(badge.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(11.0), Color32::WHITE);

                // Anchor handles (corners + side midpoints)
                for (_, p, _) in Self::handles(sel) {
                    painter.circle_filled(p, 5.0, Color32::WHITE);
                    painter.circle_stroke(p, 5.0, Stroke::new(2.0_f32, ACCENT));
                }

                if let Some((bar, cancel, tick)) = bar {
                    if matches!(self.drag_mode, DragMode::None) {
                        let hov = |r: Rect| pointer.map_or(false, |p| r.contains(p));
                        painter.rect_filled(bar, Rounding::same(21.0), Color32::from_rgb(32, 32, 40));
                        painter.rect_stroke(bar, Rounding::same(21.0), Stroke::new(1.2_f32, Color32::from_rgb(60, 60, 72)));
                        let cancel_bg = if hov(cancel) { Color32::from_rgb(224, 27, 36) } else { Color32::from_rgb(50, 50, 60) };
                        painter.circle_filled(cancel.center(), 15.0, cancel_bg);
                        painter.text(cancel.center(), egui::Align2::CENTER_CENTER, "✕", egui::FontId::proportional(15.0), Color32::WHITE);
                        let tick_bg = if hov(tick) { Color32::from_rgb(58, 215, 142) } else { Color32::from_rgb(46, 194, 126) };
                        painter.circle_filled(tick.center(), 15.0, tick_bg);
                        painter.text(tick.center(), egui::Align2::CENTER_CENTER, "✔", egui::FontId::proportional(16.0), Color32::WHITE);
                    }
                }
            }
            _ => {
                painter.rect_filled(screen, Rounding::ZERO, Color32::from_black_alpha(60));
                let banner = Rect::from_center_size(Pos2::new(screen.center().x, screen.min.y + 48.0), Vec2::new(440.0, 36.0));
                painter.rect_filled(banner, Rounding::same(18.0), Color32::from_black_alpha(200));
                painter.rect_stroke(banner, Rounding::same(18.0), Stroke::new(1.0_f32, Color32::from_white_alpha(40)));
                painter.text(
                    banner.center(),
                    egui::Align2::CENTER_CENTER,
                    "📸 Clique e arraste para selecionar uma região • Esc para cancelar",
                    egui::FontId::proportional(13.5),
                    Color32::WHITE,
                );
            }
        }
    }
}

/// Shows the live-screen overlay and returns the confirmed selection (`None` if cancelled).
pub fn run_region_selector() -> Result<Option<RegionSelection>, eframe::Error> {
    #[cfg(target_os = "linux")]
    {
        match crate::platform::linux::capture::select_region_live() {
            Ok(selection) => return Ok(selection),
            Err(e) => {
                tracing::warn!("GTK live region selector failed ({e:?}); falling back to eframe overlay");
            }
        }
    }

    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("BenderShot - Selecionar Região")
        .with_app_id("bendershot-region")
        .with_maximized(true)
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top();
    if let Some(icon) = crate::ui::load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        "bendershot",
        options,
        Box::new(move |_cc| Ok(Box::new(RegionSelectorApp::new(result_clone)))),
    )?;

    let selection = result.lock().unwrap().take();
    Ok(selection)
}
