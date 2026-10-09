use anyhow::{Context, Result};
use arboard::{Clipboard, ImageData};
use chrono::Local;
use directories::UserDirs;
use image::RgbaImage;
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

pub struct Exporter;

impl Exporter {
    pub fn copy_to_clipboard(image: &RgbaImage) -> Result<()> {
        let (width, height) = image.dimensions();
        let bytes = image.as_raw();

        let mut clipboard = Clipboard::new().context("Failed to initialize clipboard")?;
        clipboard
            .set_image(ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::Borrowed(bytes),
            })
            .context("Failed to write image to clipboard")?;

        tracing::info!("Image successfully copied to system clipboard");
        Ok(())
    }

    pub fn auto_save(image: &RgbaImage) -> Result<PathBuf> {
        let save_dir = Self::default_screenshot_dir()?;
        fs::create_dir_all(&save_dir)
            .with_context(|| format!("Failed to create screenshot dir {:?}", save_dir))?;

        let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
        let filename = format!("screenshot_{}.png", timestamp);
        let path = save_dir.join(filename);

        image
            .save(&path)
            .with_context(|| format!("Failed to auto-save screenshot to {:?}", path))?;

        tracing::info!("Screenshot auto-saved to {:?}", path);
        Ok(path)
    }

    #[allow(dead_code)]
    pub fn save_to_file(image: &RgbaImage, target: &std::path::Path) -> Result<()> {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        image
            .save(target)
            .with_context(|| format!("Failed to save screenshot to {:?}", target))?;
        Ok(())
    }

    pub fn default_screenshot_dir() -> Result<PathBuf> {
        if let Some(user_dirs) = UserDirs::new() {
            if let Some(pics) = user_dirs.picture_dir() {
                return Ok(pics.join("Screenshots"));
            }
            return Ok(user_dirs.home_dir().join("Pictures").join("Screenshots"));
        }
        Ok(PathBuf::from("Screenshots"))
    }
}
