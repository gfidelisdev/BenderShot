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
