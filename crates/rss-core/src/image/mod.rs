//! Image import, transform, and validation for Supply Signal icons.

use std::io::Cursor;
use std::path::Path;

use image::imageops::{self, FilterType};
use image::{DynamicImage, ImageFormat, RgbaImage};

use crate::error::{RssError, RssResult};
use crate::{ALLOWED_ICON_SIZES, DEFAULT_ICON_SIZE, MAX_ICON_BYTES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotateDegrees {
    None,
    Deg90,
    Deg180,
    Deg270,
}

#[derive(Debug, Clone)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct ImageProcessOptions {
    pub target_size: u32,
    pub crop: Option<CropRect>,
    pub rotate: RotateDegrees,
    pub max_bytes: u64,
}

impl Default for ImageProcessOptions {
    fn default() -> Self {
        Self {
            target_size: DEFAULT_ICON_SIZE,
            crop: None,
            rotate: RotateDegrees::None,
            max_bytes: MAX_ICON_BYTES,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageValidation {
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub format: String,
    pub square: bool,
    pub power_of_two: bool,
    pub allowed_size: bool,
    pub under_size_limit: bool,
    pub compatible: bool,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ProcessedImage {
    pub rgba: RgbaImage,
    pub png_bytes: Vec<u8>,
    pub validation: ImageValidation,
}

pub fn load_image(path: &Path) -> RssResult<DynamicImage> {
    let img = image::open(path).map_err(|e| {
        RssError::Image(format!("failed to open {}: {e}", path.display()))
    })?;
    Ok(img)
}

pub fn load_image_bytes(bytes: &[u8]) -> RssResult<DynamicImage> {
    let img = image::load_from_memory(bytes)?;
    Ok(img)
}

pub fn center_square_crop(img: &DynamicImage) -> DynamicImage {
    let w = img.width();
    let h = img.height();
    let side = w.min(h);
    let x = (w - side) / 2;
    let y = (h - side) / 2;
    img.crop_imm(x, y, side, side)
}

pub fn apply_rotate(img: DynamicImage, rotate: RotateDegrees) -> DynamicImage {
    match rotate {
        RotateDegrees::None => img,
        RotateDegrees::Deg90 => img.rotate90(),
        RotateDegrees::Deg180 => img.rotate180(),
        RotateDegrees::Deg270 => img.rotate270(),
    }
}

pub fn encode_png(img: &RgbaImage) -> RssResult<Vec<u8>> {
    let mut buf = Vec::new();
    let dyn_img = DynamicImage::ImageRgba8(img.clone());
    dyn_img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)?;
    Ok(buf)
}

/// Encode PNG and iteratively reduce dimensions if over `max_bytes`.
pub fn encode_png_under_limit(img: &RgbaImage, max_bytes: u64) -> RssResult<(RgbaImage, Vec<u8>)> {
    let mut current = img.clone();
    let mut sizes = ALLOWED_ICON_SIZES
        .iter()
        .copied()
        .filter(|&s| s <= current.width())
        .rev()
        .collect::<Vec<_>>();

    if sizes.is_empty() {
        sizes.push(current.width());
    }

    for size in sizes {
        if current.width() != size || current.height() != size {
            let resized = imageops::resize(&current, size, size, FilterType::Lanczos3);
            current = resized;
        }
        let bytes = encode_png(&current)?;
        if (bytes.len() as u64) <= max_bytes {
            return Ok((current, bytes));
        }
    }

    // Last resort: keep smallest allowed and return even if over limit (caller validates).
    let bytes = encode_png(&current)?;
    Ok((current, bytes))
}

pub fn process_image(img: DynamicImage, options: &ImageProcessOptions) -> RssResult<ProcessedImage> {
    if !ALLOWED_ICON_SIZES.contains(&options.target_size) {
        return Err(RssError::Validation(format!(
            "target size {} is not allowed; use one of {:?}",
            options.target_size, ALLOWED_ICON_SIZES
        )));
    }

    let mut working = img;

    if let Some(crop) = &options.crop {
        if crop.width == 0 || crop.height == 0 {
            return Err(RssError::Validation("crop width/height must be > 0".into()));
        }
        if crop.x + crop.width > working.width() || crop.y + crop.height > working.height() {
            return Err(RssError::Validation("crop rect is out of bounds".into()));
        }
        working = working.crop_imm(crop.x, crop.y, crop.width, crop.height);
    } else if working.width() != working.height() {
        working = center_square_crop(&working);
    }

    working = apply_rotate(working, options.rotate);

    let resized = working.resize_exact(
        options.target_size,
        options.target_size,
        FilterType::Lanczos3,
    );
    let rgba = resized.to_rgba8();
    let (final_img, png_bytes) = encode_png_under_limit(&rgba, options.max_bytes)?;
    let validation = validate_png_bytes(&png_bytes)?;

    Ok(ProcessedImage {
        rgba: final_img,
        png_bytes,
        validation,
    })
}

pub fn process_file(path: &Path, options: &ImageProcessOptions) -> RssResult<ProcessedImage> {
    let img = load_image(path)?;
    process_image(img, options)
}

pub fn validate_png_bytes(bytes: &[u8]) -> RssResult<ImageValidation> {
    let img = image::load_from_memory(bytes)?;
    let width = img.width();
    let height = img.height();
    let square = width == height;
    let power_of_two = width.is_power_of_two() && height.is_power_of_two();
    let allowed_size = ALLOWED_ICON_SIZES.contains(&width) && square;
    let under_size_limit = (bytes.len() as u64) <= MAX_ICON_BYTES;

    let mut messages = Vec::new();
    if !square {
        messages.push("image must be square".into());
    }
    if !power_of_two {
        messages.push("dimensions should be power-of-two".into());
    }
    if !allowed_size {
        messages.push(format!(
            "size must be one of {:?}, got {width}x{height}",
            ALLOWED_ICON_SIZES
        ));
    }
    if !under_size_limit {
        messages.push(format!(
            "file size {} exceeds soft limit of {} bytes",
            bytes.len(),
            MAX_ICON_BYTES
        ));
    }

    let compatible = square && allowed_size && under_size_limit;

    Ok(ImageValidation {
        width,
        height,
        bytes: bytes.len() as u64,
        format: "PNG".into(),
        square,
        power_of_two,
        allowed_size,
        under_size_limit,
        compatible,
        messages,
    })
}

pub fn png_data_url(png_bytes: &[u8]) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    format!("data:image/png;base64,{b64}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn solid(size: u32, color: [u8; 4]) -> DynamicImage {
        let img = RgbaImage::from_pixel(size, size, Rgba(color));
        DynamicImage::ImageRgba8(img)
    }

    #[test]
    fn process_makes_512_compatible() {
        let img = solid(800, [255, 0, 0, 255]);
        let out = process_image(img, &ImageProcessOptions::default()).unwrap();
        assert!(out.validation.compatible);
        assert_eq!(out.validation.width, 512);
        assert_eq!(out.validation.height, 512);
    }

    #[test]
    fn center_crop_non_square() {
        let img = RgbaImage::from_pixel(400, 200, Rgba([0, 255, 0, 255]));
        let dyn_img = DynamicImage::ImageRgba8(img);
        let cropped = center_square_crop(&dyn_img);
        assert_eq!(cropped.width(), 200);
        assert_eq!(cropped.height(), 200);
    }

    #[test]
    fn reject_invalid_target_size() {
        let img = solid(256, [0, 0, 255, 255]);
        let opts = ImageProcessOptions {
            target_size: 300,
            ..Default::default()
        };
        assert!(process_image(img, &opts).is_err());
    }
}
