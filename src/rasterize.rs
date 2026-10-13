use crate::model::{AnnotationItem, FontFamily, ObfuscateKind, Scene, TextBackground};
use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};
use std::sync::OnceLock;

static SANS_FONT: OnceLock<Option<FontArc>> = OnceLock::new();
static MONO_FONT: OnceLock<Option<FontArc>> = OnceLock::new();
static SERIF_FONT: OnceLock<Option<FontArc>> = OnceLock::new();

fn load_first_available(candidates: &[&str]) -> Option<FontArc> {
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(font) = FontArc::try_from_vec(bytes) {
                return Some(font);
            }
        }
    }
    None
}

fn get_font(family: FontFamily) -> Option<&'static FontArc> {
    match family {
        FontFamily::Sans => SANS_FONT
            .get_or_init(|| {
                let candidates = [
                    "/usr/share/fonts/abattis-cantarell-vf-fonts/Cantarell-VF.otf",
                    "/usr/share/fonts/google-noto/NotoSans-Regular.ttf",
                    "/usr/share/fonts/dejavu/DejaVuSans.ttf",
                    "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
                    "C:\\Windows\\Fonts\\segoeui.ttf",
                    "C:\\Windows\\Fonts\\arial.ttf",
                ];
                load_first_available(&candidates)
            })
            .as_ref(),
        FontFamily::Monospace => MONO_FONT
            .get_or_init(|| {
                let candidates = [
                    "/usr/share/fonts/google-noto/NotoSansMono-Regular.ttf",
                    "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
                    "/usr/share/fonts/liberation-mono/LiberationMono-Regular.ttf",
                    "C:\\Windows\\Fonts\\consola.ttf",
                    "C:\\Windows\\Fonts\\cour.ttf",
                ];
                load_first_available(&candidates).or_else(|| get_font(FontFamily::Sans).cloned())
            })
            .as_ref(),
        FontFamily::Serif => SERIF_FONT
            .get_or_init(|| {
                let candidates = [
                    "/usr/share/fonts/google-noto/NotoSerif-Regular.ttf",
                    "/usr/share/fonts/dejavu/DejaVuSerif.ttf",
                    "/usr/share/fonts/liberation-serif/LiberationSerif-Regular.ttf",
                    "C:\\Windows\\Fonts\\times.ttf",
                    "C:\\Windows\\Fonts\\georgia.ttf",
                ];
                load_first_available(&candidates).or_else(|| get_font(FontFamily::Sans).cloned())
            })
            .as_ref(),
    }
}

pub fn rasterize_scene(base: &RgbaImage, scene: &Scene) -> RgbaImage {
    rasterize_scene_at_time(base, scene, u64::MAX)
}

pub fn rasterize_scene_at_time(base: &RgbaImage, scene: &Scene, time_ms: u64) -> RgbaImage {
    let mut canvas = base.clone();

    // 1. Process obfuscation layers first (blur/pixelate) directly on pixels
    for (idx, item) in scene.items.iter().enumerate() {
        if time_ms != u64::MAX && !scene.is_item_active_at(idx, time_ms) {
            continue;
        }
        if let AnnotationItem::Obfuscate { min, max, kind, strength } = item {
            apply_obfuscation(&mut canvas, *min, *max, *kind, *strength);
        }
    }

    // 2. Render vector annotations over the canvas
    for (idx, item) in scene.items.iter().enumerate() {
        if time_ms != u64::MAX && !scene.is_item_active_at(idx, time_ms) {
            continue;
        }
        let progress = if time_ms == u64::MAX {
            1.0
        } else {
            scene.item_transition_progress(idx, time_ms)
        };
        let alpha = if time_ms == u64::MAX {
            1.0
        } else {
            scene.item_alpha_factor(idx, time_ms)
        };

        draw_item_with_progress(&mut canvas, item, progress, alpha);
    }

    canvas
}

fn draw_item_with_progress(canvas: &mut RgbaImage, item: &AnnotationItem, progress: f32, alpha: f32) {
    if progress <= 0.0 || alpha <= 0.0 {
        return;
    }

    match item {
        AnnotationItem::Freehand { points, color, width, is_highlighter } => {
            let mut col = *color;
            if *is_highlighter {
                col[3] = ((col[3] as f32 * 0.38) * alpha).clamp(20.0, 100.0) as u8;
            } else {
                col[3] = (col[3] as f32 * alpha).round() as u8;
            }
            if points.len() >= 2 {
                let count = ((points.len() as f32 * progress).ceil() as usize).max(2).min(points.len());
                draw_freehand(canvas, &points[..count], col, *width);
            } else if !points.is_empty() {
                draw_freehand(canvas, points, col, *width);
            }
        }
        AnnotationItem::Bezier { start, control, end, color, width, is_arrow } => {
            let mut col = *color;
            col[3] = (col[3] as f32 * alpha).round() as u8;
            draw_bezier(canvas, *start, *control, *end, col, *width, *is_arrow, progress);
        }
        AnnotationItem::Arrow { start, end, color, width } => {
            let mut col = *color;
            col[3] = (col[3] as f32 * alpha).round() as u8;
            draw_arrow_progress(canvas, *start, *end, col, *width, progress);
        }
        AnnotationItem::Rectangle { min, max, color, width, fill } => {
            let mut col = *color;
            col[3] = (col[3] as f32 * alpha).round() as u8;
            let fill_adj = fill.map(|mut f| {
                f[3] = (f[3] as f32 * alpha * progress).round() as u8;
                f
            });
            draw_rectangle_progress(canvas, *min, *max, col, *width, fill_adj, progress);
        }
        AnnotationItem::Polygon { points, color, width, fill, .. } => {
            let mut col = *color;
            col[3] = (col[3] as f32 * alpha).round() as u8;
            let fill_adj = fill.map(|mut f| {
                f[3] = (f[3] as f32 * alpha * progress).round() as u8;
                f
            });
            draw_polygon(canvas, points, col, *width, fill_adj, progress);
        }
        AnnotationItem::Ellipse { min, max, color, width, fill } => {
            let mut col = *color;
            col[3] = (col[3] as f32 * alpha).round() as u8;
            let fill_adj = fill.map(|mut f| {
                f[3] = (f[3] as f32 * alpha * progress).round() as u8;
                f
            });
            draw_ellipse_progress(canvas, *min, *max, col, *width, fill_adj, progress);
        }
        AnnotationItem::Highlight { min, max, color } => {
            let mut col = *color;
            col[3] = (col[3] as f32 * alpha).round() as u8;
            draw_highlight(canvas, *min, *max, col);
        }
        AnnotationItem::StepBadge { pos, number, color, radius } => {
            let mut col = *color;
            col[3] = (col[3] as f32 * alpha * progress).round() as u8;
            draw_step_badge(canvas, *pos, *number, col, *radius);
        }
        AnnotationItem::Text { pos, text, color, font_size, family, bg_style } => {
            let mut col = *color;
            col[3] = (col[3] as f32 * alpha * progress).round() as u8;
            draw_rendered_text(canvas, *pos, text, col, *font_size, *family, *bg_style);
        }
        AnnotationItem::Obfuscate { .. } => {}
    }
}

pub fn apply_obfuscation(
    img: &mut RgbaImage,
    min: [f32; 2],
    max: [f32; 2],
    kind: ObfuscateKind,
    strength: u32,
) {
    let (width, height) = img.dimensions();
    let x0 = (min[0].min(max[0]).max(0.0) as u32).min(width);
    let y0 = (min[1].min(max[1]).max(0.0) as u32).min(height);
    let x1 = (min[0].max(max[0]).max(0.0) as u32).min(width);
    let y1 = (min[1].max(max[1]).max(0.0) as u32).min(height);

    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let block_size = strength.max(6);

    match kind {
        ObfuscateKind::Pixelate => {
            for by in (y0..y1).step_by(block_size as usize) {
                for bx in (x0..x1).step_by(block_size as usize) {
                    let end_x = (bx + block_size).min(x1);
                    let end_y = (by + block_size).min(y1);

                    let mut r_sum = 0u64;
                    let mut g_sum = 0u64;
                    let mut b_sum = 0u64;
                    let mut a_sum = 0u64;
                    let mut count = 0u64;

                    for y in by..end_y {
                        for x in bx..end_x {
                            let p = img.get_pixel(x, y);
                            r_sum += p[0] as u64;
                            g_sum += p[1] as u64;
                            b_sum += p[2] as u64;
                            a_sum += p[3] as u64;
                            count += 1;
                        }
                    }

                    if count > 0 {
                        let avg = Rgba([
                            (r_sum / count) as u8,
                            (g_sum / count) as u8,
                            (b_sum / count) as u8,
                            (a_sum / count) as u8,
                        ]);
                        for y in by..end_y {
                            for x in bx..end_x {
                                img.put_pixel(x, y, avg);
                            }
                        }
                    }
                }
            }
        }
        ObfuscateKind::Blur => {
            let radius = (block_size / 2) as i32;
            let temp = img.clone();
            for y in y0..y1 {
                for x in x0..x1 {
                    let mut r_acc = 0u32;
                    let mut g_acc = 0u32;
                    let mut b_acc = 0u32;
                    let mut cnt = 0u32;

                    for dy in -radius..=radius {
                        for dx in -radius..=radius {
                            let nx = (x as i32 + dx).clamp(0, width as i32 - 1) as u32;
                            let ny = (y as i32 + dy).clamp(0, height as i32 - 1) as u32;
                            let p = temp.get_pixel(nx, ny);
                            r_acc += p[0] as u32;
                            g_acc += p[1] as u32;
                            b_acc += p[2] as u32;
                            cnt += 1;
                        }
                    }

                    let p = temp.get_pixel(x, y);
                    img.put_pixel(
                        x,
                        y,
                        Rgba([
                            (r_acc / cnt) as u8,
                            (g_acc / cnt) as u8,
                            (b_acc / cnt) as u8,
                            p[3],
                        ]),
                    );
                }
            }
        }
    }
}

pub fn apply_smudge_step(
    img: &mut RgbaImage,
    from: [f32; 2],
    to: [f32; 2],
    radius: f32,
    carried_color: &mut Option<[f32; 4]>,
) {
    let (w, h) = img.dimensions();
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let dist = (dx * dx + dy * dy).sqrt();
    let step_dist = (radius * 0.25).max(1.0);
    let steps = ((dist / step_dist).ceil() as usize).max(1);

    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let cx = from[0] + dx * t;
        let cy = from[1] + dy * t;

        let r_i = radius.ceil() as i32;
        let min_x = (cx as i32 - r_i).clamp(0, w as i32 - 1) as u32;
        let max_x = (cx as i32 + r_i).clamp(0, w as i32 - 1) as u32;
        let min_y = (cy as i32 - r_i).clamp(0, h as i32 - 1) as u32;
        let max_y = (cy as i32 + r_i).clamp(0, h as i32 - 1) as u32;

        if carried_color.is_none() {
            let px = (cx as u32).min(w - 1);
            let py = (cy as u32).min(h - 1);
            let p = img.get_pixel(px, py);
            *carried_color = Some([p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32]);
        }

        let carried = carried_color.as_mut().unwrap();

        let mut r_acc = carried[0];
        let mut g_acc = carried[1];
        let mut b_acc = carried[2];
        let mut count = 1.0_f32;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dist_sq = (px as f32 - cx).powi(2) + (py as f32 - cy).powi(2);
                if dist_sq <= radius * radius {
                    let d = dist_sq.sqrt();
                    let weight = ((1.0 - d / radius).max(0.0)).powi(2) * 0.55;

                    let orig = *img.get_pixel(px, py);
                    let new_r = (orig[0] as f32 * (1.0 - weight) + carried[0] * weight) as u8;
                    let new_g = (orig[1] as f32 * (1.0 - weight) + carried[1] * weight) as u8;
                    let new_b = (orig[2] as f32 * (1.0 - weight) + carried[2] * weight) as u8;

                    img.put_pixel(px, py, Rgba([new_r, new_g, new_b, orig[3]]));

                    r_acc += orig[0] as f32;
                    g_acc += orig[1] as f32;
                    b_acc += orig[2] as f32;
                    count += 1.0;
                }
            }
        }

        let local_avg_r = r_acc / count;
        let local_avg_g = g_acc / count;
        let local_avg_b = b_acc / count;

        carried[0] = carried[0] * 0.75 + local_avg_r * 0.25;
        carried[1] = carried[1] * 0.75 + local_avg_g * 0.25;
        carried[2] = carried[2] * 0.75 + local_avg_b * 0.25;
    }
}

fn draw_freehand(img: &mut RgbaImage, points: &[[f32; 2]], color: [u8; 4], width: f32) {
    if points.is_empty() {
        return;
    }
    let col = Rgba(color);
    if points.len() == 1 {
        let p = points[0];
        let r = (width / 2.0).max(1.0);
        let x0 = (p[0] - r).max(0.0) as u32;
        let y0 = (p[1] - r).max(0.0) as u32;
        let x1 = (p[0] + r).min(img.width() as f32 - 1.0) as u32;
        let y1 = (p[1] + r).min(img.height() as f32 - 1.0) as u32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if (x as f32 - p[0]).powi(2) + (y as f32 - p[1]).powi(2) <= r * r {
                    img.put_pixel(x, y, col);
                }
            }
        }
        return;
    }
    for window in points.windows(2) {
        draw_line_thick(img, window[0], window[1], col, width);
    }
}

pub fn apply_color_erase(
    img: &mut RgbaImage,
    target_color: [u8; 4],
    tolerance: f32, // 0.0 to 100.0 (Euclidean distance threshold)
) {
    let t_r = target_color[0] as f32;
    let t_g = target_color[1] as f32;
    let t_b = target_color[2] as f32;
    let threshold_sq = (tolerance * 2.55).powi(2);

    for pixel in img.pixels_mut() {
        let dr = pixel[0] as f32 - t_r;
        let dg = pixel[1] as f32 - t_g;
        let db = pixel[2] as f32 - t_b;
        let dist_sq = dr * dr + dg * dg + db * db;
        if dist_sq <= threshold_sq {
            // Erase: transparent pixel
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
}

#[allow(dead_code)]
fn draw_arrow(img: &mut RgbaImage, start: [f32; 2], end: [f32; 2], color: [u8; 4], width: f32) {
    draw_arrow_progress(img, start, end, color, width, 1.0);
}

fn draw_arrow_progress(
    img: &mut RgbaImage,
    start: [f32; 2],
    end: [f32; 2],
    color: [u8; 4],
    width: f32,
    progress: f32,
) {
    let col = Rgba(color);
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let len = (dx * dx + dy * dy).sqrt();
    let cur_end = [start[0] + dx * progress, start[1] + dy * progress];
    draw_line_thick(img, start, cur_end, col, width);

    if len < 5.0 {
        return;
    }

    let head_size = (width * 4.0).max(14.0);
    let cur_len = len * progress;

    // As requested: the arrowhead appears as soon as it can be distinguished from the line
    if cur_len >= head_size * 0.4 {
        let ux = dx / len;
        let uy = dy / len;

        let p1 = [
            cur_end[0] - head_size * ux + head_size * 0.5 * -uy,
            cur_end[1] - head_size * uy + head_size * 0.5 * ux,
        ];
        let p2 = [
            cur_end[0] - head_size * ux - head_size * 0.5 * -uy,
            cur_end[1] - head_size * uy - head_size * 0.5 * ux,
        ];

        draw_line_thick(img, cur_end, p1, col, width);
        draw_line_thick(img, cur_end, p2, col, width);
    }
}

fn draw_bezier(
    img: &mut RgbaImage,
    start: [f32; 2],
    control: [f32; 2],
    end: [f32; 2],
    color: [u8; 4],
    width: f32,
    is_arrow: bool,
    progress: f32,
) {
    let col = Rgba(color);
    let max_steps = 32;
    let actual_steps = ((max_steps as f32 * progress).ceil() as usize).max(1);

    let mut prev = start;
    let mut last_tangent = [end[0] - control[0], end[1] - control[1]];

    for s in 1..=actual_steps {
        let t = (s as f32 / max_steps as f32).min(progress);
        let u = 1.0 - t;
        let x = u * u * start[0] + 2.0 * u * t * control[0] + t * t * end[0];
        let y = u * u * start[1] + 2.0 * u * t * control[1] + t * t * end[1];
        let cur = [x, y];
        draw_line_thick(img, prev, cur, col, width);

        let dx = 2.0 * (1.0 - t) * (control[0] - start[0]) + 2.0 * t * (end[0] - control[0]);
        let dy = 2.0 * (1.0 - t) * (control[1] - start[1]) + 2.0 * t * (end[1] - control[1]);
        if dx * dx + dy * dy > 0.001 {
            last_tangent = [dx, dy];
        }

        prev = cur;
    }

    if is_arrow && progress > 0.15 {
        let head_size = (width * 4.0).max(14.0);
        let len = (last_tangent[0] * last_tangent[0] + last_tangent[1] * last_tangent[1]).sqrt();
        if len > 0.001 {
            let ux = last_tangent[0] / len;
            let uy = last_tangent[1] / len;
            let tip = prev;
            let p1 = [
                tip[0] - head_size * ux + head_size * 0.5 * -uy,
                tip[1] - head_size * uy + head_size * 0.5 * ux,
            ];
            let p2 = [
                tip[0] - head_size * ux - head_size * 0.5 * -uy,
                tip[1] - head_size * uy - head_size * 0.5 * ux,
            ];
            draw_line_thick(img, tip, p1, col, width);
            draw_line_thick(img, tip, p2, col, width);
        }
    }
}

fn draw_polygon(
    img: &mut RgbaImage,
    points: &[[f32; 2]],
    color: [u8; 4],
    width: f32,
    _fill: Option<[u8; 4]>,
    progress: f32,
) {
    if points.len() < 2 {
        return;
    }
    let col = Rgba(color);
    let total_edges = points.len();
    let max_edge = ((total_edges as f32 * progress).ceil() as usize).min(total_edges);

    for i in 0..max_edge {
        let p1 = points[i];
        let p2 = points[(i + 1) % total_edges];
        let edge_prog = if max_edge == i + 1 {
            let seg_start = i as f32 / total_edges as f32;
            let seg_len = 1.0 / total_edges as f32;
            ((progress - seg_start) / seg_len).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let p_end = [
            p1[0] + (p2[0] - p1[0]) * edge_prog,
            p1[1] + (p2[1] - p1[1]) * edge_prog,
        ];
        draw_line_thick(img, p1, p_end, col, width);
    }
}

fn draw_rectangle(
    img: &mut RgbaImage,
    min: [f32; 2],
    max: [f32; 2],
    color: [u8; 4],
    width: f32,
    fill: Option<[u8; 4]>,
) {
    draw_rectangle_progress(img, min, max, color, width, fill, 1.0);
}

fn draw_rectangle_progress(
    img: &mut RgbaImage,
    min: [f32; 2],
    max: [f32; 2],
    color: [u8; 4],
    width: f32,
    fill: Option<[u8; 4]>,
    progress: f32,
) {
    let x0 = min[0].min(max[0]);
    let y0 = min[1].min(max[1]);
    let x1 = min[0].max(max[0]);
    let y1 = min[1].max(max[1]);

    if let Some(f_col) = fill {
        let f = Rgba(f_col);
        for y in (y0 as u32)..(y1 as u32).min(img.height()) {
            for x in (x0 as u32)..(x1 as u32).min(img.width()) {
                blend_pixel(img, x, y, f);
            }
        }
    }

    let pts = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
    draw_polygon(img, &pts, color, width, None, progress);
}

fn draw_ellipse_progress(
    img: &mut RgbaImage,
    min: [f32; 2],
    max: [f32; 2],
    color: [u8; 4],
    width: f32,
    fill: Option<[u8; 4]>,
    progress: f32,
) {
    let cx = (min[0] + max[0]) / 2.0;
    let cy = (min[1] + max[1]) / 2.0;
    let rx = ((max[0] - min[0]).abs() / 2.0).max(1.0);
    let ry = ((max[1] - min[1]).abs() / 2.0).max(1.0);

    if let Some(f_col) = fill {
        let (w, h) = img.dimensions();
        let min_x = (cx - rx).max(0.0) as u32;
        let max_x = (cx + rx).min(w as f32) as u32;
        let min_y = (cy - ry).max(0.0) as u32;
        let max_y = (cy + ry).min(h as f32) as u32;
        let f = Rgba(f_col);
        for y in min_y..max_y {
            for x in min_x..max_x {
                let dx = (x as f32 - cx) / rx;
                let dy = (y as f32 - cy) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    blend_pixel(img, x, y, f);
                }
            }
        }
    }

    let total_steps = 48;
    let actual_steps = ((total_steps as f32 * progress).ceil() as usize).max(1);
    let col = Rgba(color);

    let mut prev = [cx, cy - ry];
    for i in 1..=actual_steps {
        let theta = -std::f32::consts::FRAC_PI_2 + (i as f32 / total_steps as f32) * std::f32::consts::TAU * progress;
        let cur = [cx + rx * theta.cos(), cy + ry * theta.sin()];
        draw_line_thick(img, prev, cur, col, width);
        prev = cur;
    }
}

fn draw_highlight(img: &mut RgbaImage, min: [f32; 2], max: [f32; 2], color: [u8; 4]) {
    let (w, h) = img.dimensions();
    let x0 = (min[0].min(max[0]).max(0.0) as u32).min(w);
    let y0 = (min[1].min(max[1]).max(0.0) as u32).min(h);
    let x1 = (min[0].max(max[0]).max(0.0) as u32).min(w);
    let y1 = (min[1].max(max[1]).max(0.0) as u32).min(h);

    if x0 >= x1 || y0 >= y1 {
        return;
    }

    // Alpha blend with 35% opacity
    let alpha = 0.38_f32;
    let inv_alpha = 1.0 - alpha;
    let h_r = color[0] as f32;
    let h_g = color[1] as f32;
    let h_b = color[2] as f32;

    for y in y0..y1 {
        for x in x0..x1 {
            let orig = *img.get_pixel(x, y);
            let nr = (orig[0] as f32 * inv_alpha + h_r * alpha) as u8;
            let ng = (orig[1] as f32 * inv_alpha + h_g * alpha) as u8;
            let nb = (orig[2] as f32 * inv_alpha + h_b * alpha) as u8;
            img.put_pixel(x, y, Rgba([nr, ng, nb, orig[3]]));
        }
    }
}

#[allow(dead_code)]
fn draw_ellipse(
    img: &mut RgbaImage,
    min: [f32; 2],
    max: [f32; 2],
    color: [u8; 4],
    width: f32,
    fill: Option<[u8; 4]>,
) {
    let cx = (min[0] + max[0]) / 2.0;
    let cy = (min[1] + max[1]) / 2.0;
    let rx = ((max[0] - min[0]).abs() / 2.0).max(1.0);
    let ry = ((max[1] - min[1]).abs() / 2.0).max(1.0);

    let (w, h) = img.dimensions();
    let min_x = (cx - rx - width).max(0.0) as u32;
    let max_x = (cx + rx + width).min(w as f32) as u32;
    let min_y = (cy - ry - width).max(0.0) as u32;
    let max_y = (cy + ry + width).min(h as f32) as u32;

    let col = Rgba(color);
    let fill_col = fill.map(Rgba);

    for y in min_y..max_y {
        for x in min_x..max_x {
            let dx = (x as f32 - cx) / rx;
            let dy = (y as f32 - cy) / ry;
            let d = dx * dx + dy * dy;

            if d <= 1.0 {
                if let Some(fc) = fill_col {
                    img.put_pixel(x, y, fc);
                }
            }

            let border_thickness = width / rx.min(ry);
            if (d - 1.0).abs() <= border_thickness {
                img.put_pixel(x, y, col);
            }
        }
    }
}

fn draw_step_badge(
    img: &mut RgbaImage,
    pos: [f32; 2],
    number: u32,
    color: [u8; 4],
    radius: f32,
) {
    let (w, h) = img.dimensions();
    let min_x = (pos[0] - radius).max(0.0) as u32;
    let max_x = (pos[0] + radius).min(w as f32) as u32;
    let min_y = (pos[1] - radius).max(0.0) as u32;
    let max_y = (pos[1] + radius).min(h as f32) as u32;

    let badge_color = Rgba(color);

    for y in min_y..max_y {
        for x in min_x..max_x {
            let dx = x as f32 - pos[0];
            let dy = y as f32 - pos[1];
            if dx * dx + dy * dy <= radius * radius {
                img.put_pixel(x, y, badge_color);
            }
        }
    }

    let text = number.to_string();
    let text_x = pos[0] - (radius * 0.35);
    let text_y = pos[1] - (radius * 0.45);
    draw_rendered_text(
        img,
        [text_x, text_y],
        &text,
        [255, 255, 255, 255],
        radius * 1.0,
        FontFamily::Sans,
        TextBackground::Halo,
    );
}

fn draw_rendered_text(
    img: &mut RgbaImage,
    pos: [f32; 2],
    text: &str,
    color: [u8; 4],
    font_size: f32,
    family: FontFamily,
    bg_style: TextBackground,
) {
    if let Some(font) = get_font(family) {
        let scale = PxScale::from(font_size);
        let scaled_font = font.as_scaled(scale);

        if bg_style == TextBackground::FilledTag {
            let mut width = 0.0_f32;
            for ch in text.chars() {
                let glyph_id = scaled_font.glyph_id(ch);
                width += scaled_font.h_advance(glyph_id);
            }
            let pad_x = 8.0_f32;
            let pad_y = 4.0_f32;
            let box_min = [pos[0] - pad_x, pos[1] - pad_y];
            let box_max = [pos[0] + width + pad_x, pos[1] + font_size + pad_y];
            draw_rectangle(img, box_min, box_max, [40, 40, 40, 255], 1.0, Some([24, 24, 24, 220]));
        } else {
            let halo_color = [0u8, 0u8, 0u8, 220u8];
            for (dx, dy) in [(-1.5, 0.0), (1.5, 0.0), (0.0, -1.5), (0.0, 1.5), (1.0, 1.0), (-1.0, -1.0)] {
                render_glyph_string(img, scaled_font, [pos[0] + dx, pos[1] + dy], text, halo_color, scale);
            }
        }

        render_glyph_string(img, scaled_font, pos, text, color, scale);
    } else {
        draw_simple_text_fallback(img, pos, text, color, font_size);
    }
}

fn render_glyph_string<F: Font>(
    img: &mut RgbaImage,
    scaled_font: ab_glyph::PxScaleFont<F>,
    pos: [f32; 2],
    text: &str,
    color: [u8; 4],
    scale: PxScale,
) {
    let mut cursor_x = pos[0];
    let cursor_y = pos[1] + scaled_font.ascent();

    for ch in text.chars() {
        let glyph_id = scaled_font.glyph_id(ch);
        let glyph = glyph_id.with_scale_and_position(scale, ab_glyph::point(cursor_x, cursor_y));

        if let Some(outlined) = scaled_font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, coverage| {
                let px = bounds.min.x as i32 + x as i32;
                let py = bounds.min.y as i32 + y as i32;
                if px >= 0 && px < img.width() as i32 && py >= 0 && py < img.height() as i32 {
                    let orig = img.get_pixel(px as u32, py as u32);
                    let alpha = (color[3] as f32 / 255.0) * coverage;
                    let r = (color[0] as f32 * alpha + orig[0] as f32 * (1.0 - alpha)) as u8;
                    let g = (color[1] as f32 * alpha + orig[1] as f32 * (1.0 - alpha)) as u8;
                    let b = (color[2] as f32 * alpha + orig[2] as f32 * (1.0 - alpha)) as u8;
                    img.put_pixel(px as u32, py as u32, Rgba([r, g, b, 255]));
                }
            });
        }
        cursor_x += scaled_font.h_advance(glyph_id);
    }
}

fn draw_simple_text_fallback(
    img: &mut RgbaImage,
    pos: [f32; 2],
    text: &str,
    color: [u8; 4],
    font_size: f32,
) {
    let col = Rgba(color);
    let scale = (font_size / 8.0).max(1.0);
    let mut cur_x = pos[0];

    for c in text.chars() {
        let pattern = get_char_5x7(c);
        for row in 0..7 {
            for col_idx in 0..5 {
                if (pattern[row] >> (4 - col_idx)) & 1 == 1 {
                    for dy in 0..(scale as u32) {
                        for dx in 0..(scale as u32) {
                            let px = (cur_x + (col_idx as f32 * scale) + dx as f32) as i32;
                            let py = (pos[1] + (row as f32 * scale) + dy as f32) as i32;
                            if px >= 0 && px < img.width() as i32 && py >= 0 && py < img.height() as i32 {
                                img.put_pixel(px as u32, py as u32, col);
                            }
                        }
                    }
                }
            }
        }
        cur_x += 6.0 * scale;
    }
}

pub fn blend_pixel(img: &mut RgbaImage, x: u32, y: u32, col: Rgba<u8>) {
    if col[3] == 255 {
        img.put_pixel(x, y, col);
    } else if col[3] > 0 {
        let orig = img.get_pixel(x, y);
        let a = col[3] as f32 / 255.0;
        let r = (col[0] as f32 * a + orig[0] as f32 * (1.0 - a)).round() as u8;
        let g = (col[1] as f32 * a + orig[1] as f32 * (1.0 - a)).round() as u8;
        let b = (col[2] as f32 * a + orig[2] as f32 * (1.0 - a)).round() as u8;
        let out_a = orig[3].max(col[3]);
        img.put_pixel(x, y, Rgba([r, g, b, out_a]));
    }
}

fn draw_line_thick(img: &mut RgbaImage, start: [f32; 2], end: [f32; 2], col: Rgba<u8>, width: f32) {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let steps = (dx.abs().max(dy.abs()) * 2.0).max(1.0) as usize;
    let half = (width / 2.0).max(0.5);

    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let x = start[0] + dx * t;
        let y = start[1] + dy * t;

        for ox in (-half as i32)..=(half as i32) {
            for oy in (-half as i32)..=(half as i32) {
                let px = (x as i32 + ox).clamp(0, img.width() as i32 - 1) as u32;
                let py = (y as i32 + oy).clamp(0, img.height() as i32 - 1) as u32;
                blend_pixel(img, px, py, col);
            }
        }
    }
}

fn get_char_5x7(c: char) -> [u8; 7] {
    match c {
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        _ => [0b11111, 0b10001, 0b10001, 0b11111, 0b10000, 0b10000, 0b10000],
    }
}
