use crate::model::*;
use crate::rasterize::rasterize_scene;
use image::{Rgba, RgbaImage};

#[test]
fn test_scene_badges_and_recalc() {
    let mut scene = Scene::new();
    assert_eq!(scene.next_badge_number, 1);

    scene.push_item(AnnotationItem::StepBadge {
        pos: [50.0, 50.0],
        number: 1,
        color: [255, 0, 0, 255],
        radius: 12.0,
    });
    assert_eq!(scene.next_badge_number, 2);
    assert_eq!(scene.items.len(), 1);

    scene.push_item(AnnotationItem::StepBadge {
        pos: [80.0, 80.0],
        number: 2,
        color: [255, 0, 0, 255],
        radius: 12.0,
    });
    assert_eq!(scene.next_badge_number, 3);
    assert_eq!(scene.items.len(), 2);

    // Test removing item and recalculating badges
    scene.items.pop();
    scene.recalc_badge_number();
    assert_eq!(scene.items.len(), 1);
    assert_eq!(scene.next_badge_number, 2);

    // Test Clear
    scene.clear();
    assert_eq!(scene.items.len(), 0);
    assert_eq!(scene.next_badge_number, 1);
}

#[test]
fn test_rasterize_obfuscation_and_shapes() {
    let mut img = RgbaImage::new(100, 100);
    for pixel in img.pixels_mut() {
        *pixel = Rgba([100, 150, 200, 255]);
    }

    let mut scene = Scene::new();
    scene.push_item(AnnotationItem::Rectangle {
        min: [10.0, 10.0],
        max: [40.0, 40.0],
        color: [255, 0, 0, 255],
        width: 2.0,
        fill: None,
    });
    scene.push_item(AnnotationItem::Obfuscate {
        min: [50.0, 50.0],
        max: [80.0, 80.0],
        kind: ObfuscateKind::Pixelate,
        strength: 8,
    });

    let rendered = rasterize_scene(&img, &scene);
    assert_eq!(rendered.dimensions(), (100, 100));

    // Pixel inside rectangle stroke should be red
    let rect_stroke_pixel = rendered.get_pixel(10, 10);
    assert_eq!(rect_stroke_pixel[0], 255); // Red component
}

#[test]
fn test_smudge_step() {
    let mut img = RgbaImage::new(50, 50);
    // Half black, half white
    for y in 0..50 {
        for x in 0..50 {
            if x < 25 {
                img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            } else {
                img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
    }

    let mut carried = None;
    // Smudge from black side across into white side
    crate::rasterize::apply_smudge_step(&mut img, [20.0, 25.0], [35.0, 25.0], 6.0, &mut carried);

    // Pixel at (30, 25) should now be smeared (darker than pure white)
    let smeared = img.get_pixel(30, 25);
    assert!(smeared[0] < 250, "Pixel was not smeared: {:?}", smeared);
}

#[test]
fn test_highlight_rasterize() {
    let mut img = RgbaImage::new(40, 40);
    for pixel in img.pixels_mut() {
        *pixel = Rgba([255, 255, 255, 255]); // Pure white
    }

    let mut scene = Scene::new();
    // Highlight with Yellow [255, 255, 0, 255]
    scene.push_item(AnnotationItem::Highlight {
        min: [10.0, 10.0],
        max: [30.0, 30.0],
        color: [255, 255, 0, 255],
    });

    let rendered = rasterize_scene(&img, &scene);
    let highlighted_pixel = rendered.get_pixel(20, 20);
    // Yellow highlight over white lowers blue channel (since yellow has 0 blue)
    assert!(highlighted_pixel[2] < 200, "Blue channel should be reduced by yellow highlight: {:?}", highlighted_pixel);
    assert_eq!(highlighted_pixel[0], 255); // Red remains high
}

#[test]
fn test_freehand_rasterize() {
    let mut img = RgbaImage::new(40, 40);
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 255]); // Black canvas
    }

    let mut scene = Scene::new();
    scene.push_item(AnnotationItem::Freehand {
        points: vec![[5.0, 5.0], [10.0, 10.0], [20.0, 20.0]],
        color: [0, 255, 0, 255], // Green
        width: 3.0,
        is_highlighter: false,
    });

    let rendered = rasterize_scene(&img, &scene);
    let p = rendered.get_pixel(10, 10);
    assert_eq!(p[1], 255, "Green channel should be 255: {:?}", p);
}

#[test]
fn test_color_erase() {
    let mut img = RgbaImage::new(30, 30);
    // Fill with Red
    for pixel in img.pixels_mut() {
        *pixel = Rgba([255, 0, 0, 255]);
    }
    // Set a blue spot at (5, 5)
    img.put_pixel(5, 5, Rgba([0, 0, 255, 255]));

    // Erase Red with tolerance 10%
    crate::rasterize::apply_color_erase(&mut img, [255, 0, 0, 255], 10.0);

    // Previously red pixels should now be erased (alpha 0)
    let erased = img.get_pixel(15, 15);
    assert_eq!(erased[3], 0, "Red pixel should be erased (alpha 0): {:?}", erased);

    // Blue pixel should remain intact
    let preserved = img.get_pixel(5, 5);
    assert_eq!(preserved[3], 255, "Blue pixel should not be erased: {:?}", preserved);
    assert_eq!(preserved[2], 255);
}

#[test]
fn test_annotation_span_and_scene_timeline() {
    let mut scene = Scene::new();
    // Item 0: Permanent (no span)
    scene.push_item(AnnotationItem::Rectangle {
        min: [0.0, 0.0],
        max: [10.0, 10.0],
        color: [255, 0, 0, 255],
        width: 2.0,
        fill: None,
    });

    // Item 1: Temporary (active only between 1000ms and 3000ms)
    let span = AnnotationSpan::new(1000, 3000)
        .with_draw_transition(500)
        .with_fade_out(200);

    scene.push_item_with_span(
        AnnotationItem::Highlight {
            min: [20.0, 20.0],
            max: [30.0, 30.0],
            color: [255, 255, 0, 255],
        },
        Some(span),
    );

    // Item 0 is always active
    assert!(scene.is_item_active_at(0, 500));
    assert!(scene.is_item_active_at(0, 2000));
    assert!(scene.is_item_active_at(0, 4000));

    // Item 1 is active only during [1000, 3000]
    assert!(!scene.is_item_active_at(1, 500));
    assert!(scene.is_item_active_at(1, 1000));
    assert!(scene.is_item_active_at(1, 2000));
    assert!(scene.is_item_active_at(1, 3000));
    assert!(!scene.is_item_active_at(1, 3500));

    // Test progressive write-on animation progress
    assert_eq!(span.progress_at(1000), 0.0);
    assert!((span.progress_at(1250) - 0.5).abs() < 0.01);
    assert_eq!(span.progress_at(1500), 1.0);
    assert_eq!(span.progress_at(2500), 1.0);

    // Test fade-out exit alpha factor
    assert_eq!(span.alpha_factor_at(2500), 1.0);
    assert!((span.alpha_factor_at(2900) - 0.5).abs() < 0.01);
    assert_eq!(span.alpha_factor_at(3000), 0.0);
}

#[test]
fn test_stroke_smoothing() {
    let raw = vec![
        [0.0, 0.0],
        [1.0, 2.0],
        [2.0, 4.0],
        [10.0, 20.0],
        [30.0, 60.0],
    ];
    let smoothed = crate::smooth::smooth_stroke(&raw, 2);
    assert!(smoothed.len() >= raw.len(), "Smoothed stroke should have interpolated points");
    // Start and end points preserved
    assert_eq!(smoothed[0], raw[0]);
    assert_eq!(*smoothed.last().unwrap(), *raw.last().unwrap());
}

#[test]
fn test_bezier_and_polygon_rasterize() {
    let mut img = RgbaImage::new(60, 60);
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 255]);
    }

    let mut scene = Scene::new();
    scene.push_item(AnnotationItem::Bezier {
        start: [5.0, 5.0],
        control: [25.0, 5.0],
        end: [25.0, 25.0],
        color: [255, 0, 0, 255],
        width: 3.0,
        is_arrow: true,
    });
    scene.push_item(AnnotationItem::Polygon {
        points: vec![[35.0, 35.0], [55.0, 35.0], [45.0, 55.0]],
        sides: 3,
        color: [0, 0, 255, 255],
        width: 3.0,
        fill: None,
    });

    let rendered = rasterize_scene(&img, &scene);
    let p_bez = rendered.get_pixel(5, 5);
    assert_eq!(p_bez[0], 255, "Red channel at start of bezier: {:?}", p_bez);

    let p_poly = rendered.get_pixel(35, 35);
    assert_eq!(p_poly[2], 255, "Blue channel at vertex of polygon: {:?}", p_poly);
}

#[test]
fn test_regular_polygon_ctrl_equal_sides_and_radii() {
    // When is_ctrl is true, rx == ry == r
    let cx = 100.0_f32;
    let cy = 100.0_f32;
    let r = 50.0_f32;

    for sides in [3, 4, 5, 6, 8, 12] {
        let pts: Vec<[f32; 2]> = (0..sides)
            .map(|i| {
                let theta = -std::f32::consts::FRAC_PI_2
                    + (i as f32 / sides as f32) * std::f32::consts::TAU;
                [cx + r * theta.cos(), cy + r * theta.sin()]
            })
            .collect();

        // 1. All vertex distances to center are equal to r
        for p in &pts {
            let dx = p[0] - cx;
            let dy = p[1] - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            assert!((dist - r).abs() < 1e-4, "Vertex distance to center must equal r={}: got {}", r, dist);
        }

        // 2. All side lengths between consecutive vertices are equal
        let side_len_0 = {
            let p0 = pts[0];
            let p1 = pts[1];
            let dx = p1[0] - p0[0];
            let dy = p1[1] - p0[1];
            (dx * dx + dy * dy).sqrt()
        };

        for i in 0..sides {
            let p_a = pts[i];
            let p_b = pts[(i + 1) % sides];
            let dx = p_b[0] - p_a[0];
            let dy = p_b[1] - p_a[1];
            let side_len = (dx * dx + dy * dy).sqrt();
            assert!(
                (side_len - side_len_0).abs() < 1e-4,
                "All sides of regular {}-gon must be equal: side 0={}, side {}={}",
                sides, side_len_0, i, side_len
            );
        }
    }
}

#[test]
fn test_cross_platform_paths() {
    let pid_path = crate::platform::imp::video::pid_file_path();
    let temp_video = crate::platform::imp::video::temp_video_path();
    let path_file = crate::platform::imp::video::video_path_file();

    let temp_dir = std::env::temp_dir();
    assert!(pid_path.starts_with(&temp_dir), "pid_path {:?} must be in temp_dir {:?}", pid_path, temp_dir);
    assert!(temp_video.starts_with(&temp_dir), "temp_video {:?} must be in temp_dir {:?}", temp_video, temp_dir);
    assert!(path_file.starts_with(&temp_dir), "path_file {:?} must be in temp_dir {:?}", path_file, temp_dir);
}

#[test]
fn test_shortcut_manager_defaults() {
    let binding = crate::shortcuts::ShortcutManager::get_current_binding();
    assert!(!binding.is_empty(), "Default screenshot binding should not be empty");

    let record_binding = crate::shortcuts::ShortcutManager::get_current_record_binding();
    assert!(!record_binding.is_empty(), "Default record binding should not be empty");
}

#[test]
fn test_export_default_directory() {
    let dir = crate::export::Exporter::default_screenshot_dir();
    assert!(dir.is_ok(), "default_screenshot_dir must resolve successfully");
}

#[cfg(target_os = "windows")]
#[test]
fn test_windows_parse_hotkey() {
    use crate::platform::windows::shortcuts::ShortcutManager;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MOD_CONTROL, MOD_SHIFT, MOD_ALT, MOD_WIN, MOD_NOREPEAT, VK_SNAPSHOT
    };

    let p1 = ShortcutManager::parse_hotkey("<Ctrl><Shift>S").expect("Should parse <Ctrl><Shift>S");
    assert_eq!(p1.0, MOD_NOREPEAT | MOD_CONTROL | MOD_SHIFT);
    assert_eq!(p1.1, 0x53); // 'S'

    let p2 = ShortcutManager::parse_hotkey("Ctrl+Shift+S").expect("Should parse Ctrl+Shift+S");
    assert_eq!(p2, p1);

    let p3 = ShortcutManager::parse_hotkey("Print").expect("Should parse Print");
    assert_eq!(p3.0, MOD_NOREPEAT);
    assert_eq!(p3.1, VK_SNAPSHOT as u32);

    let p4 = ShortcutManager::parse_hotkey("<Super><Alt>R").expect("Should parse <Super><Alt>R");
    assert_eq!(p4.0, MOD_NOREPEAT | MOD_WIN | MOD_ALT);
    assert_eq!(p4.1, 0x52); // 'R'
}

