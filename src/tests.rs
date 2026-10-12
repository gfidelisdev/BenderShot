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
    scene.push_item_with_span(
        AnnotationItem::Highlight {
            min: [20.0, 20.0],
            max: [30.0, 30.0],
            color: [255, 255, 0, 255],
        },
        Some(AnnotationSpan::new(1000, 3000)),
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
}



