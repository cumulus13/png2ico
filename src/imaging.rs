//! Image processing: square fitting with alpha-correct resampling.

use crate::color::Color;
use image::{imageops, imageops::FilterType, Rgba, RgbaImage};

/// How the source image is fitted into the square icon canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fit {
    /// Keep aspect ratio, pad with transparency (or background). Default.
    #[default]
    Contain,
    /// Keep aspect ratio, fill the canvas and crop the overflow (center crop).
    Cover,
    /// Ignore aspect ratio and stretch to a square.
    Stretch,
}

fn premultiply(img: &mut RgbaImage) {
    for p in img.pixels_mut() {
        let a = u32::from(p[3]);
        if a == 255 {
            continue;
        }
        for c in 0..3 {
            p[c] = ((u32::from(p[c]) * a + 127) / 255) as u8;
        }
    }
}

fn unpremultiply(img: &mut RgbaImage) {
    for p in img.pixels_mut() {
        let a = u32::from(p[3]);
        match a {
            255 => {}
            0 => *p = Rgba([0, 0, 0, 0]),
            _ => {
                for c in 0..3 {
                    p[c] = ((u32::from(p[c]) * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
    }
}

/// Lanczos3 resize performed in premultiplied alpha space.
///
/// Resizing straight (non-premultiplied) RGBA bleeds the color of fully
/// transparent pixels into the edges, producing dark/white halos on icons.
pub fn resize_premultiplied(src: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    let (width, height) = (width.max(1), height.max(1));
    if src.dimensions() == (width, height) {
        return src.clone();
    }
    let mut pm = src.clone();
    premultiply(&mut pm);
    let mut out = imageops::resize(&pm, width, height, FilterType::Lanczos3);
    unpremultiply(&mut out);
    out
}

/// Render `src` into a `size` x `size` RGBA canvas.
///
/// `padding_pct` is the margin on each side as a percentage of `size`
/// (clamped to 0..=45).
pub fn render_square(
    src: &RgbaImage,
    size: u32,
    fit: Fit,
    padding_pct: f32,
    background: Option<Color>,
) -> RgbaImage {
    let size = size.max(1);
    let pad = ((size as f32) * padding_pct.clamp(0.0, 45.0) / 100.0).round() as u32;
    let inner = size.saturating_sub(pad * 2).max(1);
    let (sw, sh) = src.dimensions();

    let layer = match fit {
        Fit::Stretch => resize_premultiplied(src, inner, inner),
        Fit::Contain => {
            let scale = (f64::from(inner) / f64::from(sw)).min(f64::from(inner) / f64::from(sh));
            let nw = ((f64::from(sw) * scale).round() as u32).clamp(1, inner);
            let nh = ((f64::from(sh) * scale).round() as u32).clamp(1, inner);
            resize_premultiplied(src, nw, nh)
        }
        Fit::Cover => {
            let scale = (f64::from(inner) / f64::from(sw)).max(f64::from(inner) / f64::from(sh));
            let nw = ((f64::from(sw) * scale).round() as u32).max(inner);
            let nh = ((f64::from(sh) * scale).round() as u32).max(inner);
            let resized = resize_premultiplied(src, nw, nh);
            let x = (nw - inner) / 2;
            let y = (nh - inner) / 2;
            imageops::crop_imm(&resized, x, y, inner, inner).to_image()
        }
    };

    let (lw, lh) = layer.dimensions();
    let x = i64::from((size - lw) / 2);
    let y = i64::from((size - lh) / 2);

    match background {
        Some(bg) if bg.0[3] > 0 => {
            let mut canvas = RgbaImage::from_pixel(size, size, Rgba(bg.0));
            imageops::overlay(&mut canvas, &layer, x, y);
            canvas
        }
        _ => {
            let mut canvas = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
            imageops::replace(&mut canvas, &layer, x, y);
            canvas
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba([255, 0, 0, 255]))
    }

    #[test]
    fn contain_pads_with_transparency() {
        let out = render_square(&red(64, 32), 32, Fit::Contain, 0.0, None);
        assert_eq!(out.dimensions(), (32, 32));
        assert_eq!(out.get_pixel(16, 0)[3], 0, "top rows are padding");
        assert_eq!(out.get_pixel(16, 16)[3], 255, "center is opaque");
    }

    #[test]
    fn cover_fills_canvas() {
        let out = render_square(&red(64, 32), 32, Fit::Cover, 0.0, None);
        assert!(out.pixels().all(|p| p[3] == 255));
    }

    #[test]
    fn padding_shrinks_content() {
        let out = render_square(&red(32, 32), 32, Fit::Contain, 25.0, None);
        assert_eq!(out.get_pixel(0, 0)[3], 0);
        assert_eq!(out.get_pixel(16, 16)[3], 255);
    }

    #[test]
    fn background_fills_padding() {
        let bg = Color([0, 0, 255, 255]);
        let out = render_square(&red(64, 32), 32, Fit::Contain, 0.0, Some(bg));
        assert_eq!(out.get_pixel(16, 0).0, [0, 0, 255, 255]);
    }

    #[test]
    fn no_halo_on_transparent_edges() {
        // Opaque white square on a fully transparent black background.
        let mut img = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        for y in 16..48 {
            for x in 16..48 {
                img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        let out = resize_premultiplied(&img, 16, 16);
        for p in out.pixels().filter(|p| p[3] >= 128) {
            assert!(p[0] >= 250, "edge pixel darkened: {:?}", p);
        }
    }
}
