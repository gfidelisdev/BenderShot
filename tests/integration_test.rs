use bendershot::export::Exporter;
use bendershot::model::*;
use bendershot::rasterize::rasterize_scene;
use image::{Rgba, RgbaImage};
use std::fs;

#[test]
fn test_end_to_end_rasterize_and_save() {
    let mut base = RgbaImage::new(400, 300);
    for pixel in base.pixels_mut() {
        *pixel = Rgba([240, 240, 245, 255]);
    }

    let mut scene = Scene::new();

    // 1. Arrow
    scene.push_item(AnnotationItem::Arrow {
        start: [20.0, 20.0],
        end: [120.0, 100.0],
        color: [255, 0, 0, 255],
        width: 3.0,
    });

    // 2. Rectangle
    scene.push_item(AnnotationItem::Rectangle {
        min: [50.0, 50.0],
        max: [200.0, 150.0],
        color: [0, 120, 255, 255],
        width: 2.0,
        fill: None,
    });

    // 3. Step Badge ①
    scene.push_item(AnnotationItem::StepBadge {
        pos: [220.0, 80.0],
        number: 1,
        color: [230, 80, 0, 255],
        radius: 14.0,
    });

    // 4. Obfuscation (Pixelate)
    scene.push_item(AnnotationItem::Obfuscate {
        min: [100.0, 180.0],
        max: [300.0, 260.0],
        kind: ObfuscateKind::Pixelate,
        strength: 10,
    });

    // 5. Obfuscation (Blur)
    scene.push_item(AnnotationItem::Obfuscate {
        min: [310.0, 180.0],
        max: [380.0, 260.0],
        kind: ObfuscateKind::Blur,
        strength: 6,
    });

    // Rasterize
    let final_raster = rasterize_scene(&base, &scene);
    assert_eq!(final_raster.dimensions(), (400, 300));

    // Save to temp test path
    let tmp_dir = std::env::temp_dir().join("enhanced_shot_test");
    let test_file = tmp_dir.join("test_output.png");
    let save_res = Exporter::save_to_file(&final_raster, &test_file);
    assert!(save_res.is_ok(), "Failed to save rendered screenshot: {:?}", save_res);
    assert!(test_file.exists());

    // Clean up
    let _ = fs::remove_file(&test_file);
    let _ = fs::remove_dir(&tmp_dir);
}
