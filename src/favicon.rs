//! favicon.io-style package generator: PNG icons, `favicon.ico` and `site.webmanifest`.

use crate::color::Color;
use crate::convert::{emit, normalize_sizes, Plan, Written};
use crate::error::{Error, Result};
use crate::ico::{encode_ico, encode_png, Encoding};
use crate::imaging::{render_square, Fit};
use image::RgbaImage;
use rayon::prelude::*;
use std::path::Path;

/// PNG files of the package: `(file name, pixel size)`.
pub const PNG_TARGETS: &[(&str, u32)] = &[
    ("apple-touch-icon.png", 180),
    ("favicon-32x32.png", 32),
    ("favicon-16x16.png", 16),
    ("android-chrome-192x192.png", 192),
    ("android-chrome-512x512.png", 512),
];

/// Sizes stored in `favicon.ico` by default.
pub const DEFAULT_ICO_SIZES: &[u32] = &[16, 32, 48];

const APPLE_ICON: &str = "apple-touch-icon.png";
const ICO_NAME: &str = "favicon.ico";
const MANIFEST_NAME: &str = "site.webmanifest";

/// Options for [`generate_favicons`].
#[derive(Debug, Clone)]
pub struct FaviconOptions {
    pub fit: Fit,
    pub padding: f32,
    /// Background for padding. `apple-touch-icon.png` is always opaque
    /// (iOS paints transparency black), defaulting to white.
    pub background: Option<Color>,
    pub name: String,
    pub short_name: String,
    pub theme_color: String,
    pub background_color: String,
    pub ico_sizes: Vec<u32>,
    pub force: bool,
    pub dry_run: bool,
}

impl Default for FaviconOptions {
    fn default() -> Self {
        Self {
            fit: Fit::Contain,
            padding: 0.0,
            background: None,
            name: String::new(),
            short_name: String::new(),
            theme_color: "#ffffff".into(),
            background_color: "#ffffff".into(),
            ico_sizes: DEFAULT_ICO_SIZES.to_vec(),
            force: false,
            dry_run: false,
        }
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out
}

/// Contents of `site.webmanifest`.
pub fn web_manifest(opts: &FaviconOptions) -> String {
    format!(
        concat!(
            "{{\"name\":\"{}\",\"short_name\":\"{}\",\"icons\":[",
            "{{\"src\":\"/android-chrome-192x192.png\",\"sizes\":\"192x192\",\"type\":\"image/png\"}},",
            "{{\"src\":\"/android-chrome-512x512.png\",\"sizes\":\"512x512\",\"type\":\"image/png\"}}],",
            "\"theme_color\":\"{}\",\"background_color\":\"{}\",\"display\":\"standalone\"}}"
        ),
        json_escape(&opts.name),
        json_escape(&opts.short_name),
        json_escape(&opts.theme_color),
        json_escape(&opts.background_color),
    )
}

/// `<link>` tags to paste into the `<head>` of a site.
pub fn html_snippet() -> &'static str {
    concat!(
        "<link rel=\"apple-touch-icon\" sizes=\"180x180\" href=\"/apple-touch-icon.png\">\n",
        "<link rel=\"icon\" type=\"image/png\" sizes=\"32x32\" href=\"/favicon-32x32.png\">\n",
        "<link rel=\"icon\" type=\"image/png\" sizes=\"16x16\" href=\"/favicon-16x16.png\">\n",
        "<link rel=\"icon\" href=\"/favicon.ico\" sizes=\"any\">\n",
        "<link rel=\"manifest\" href=\"/site.webmanifest\">\n"
    )
}

/// Generate the full favicon package into `out_dir`.
///
/// All target paths are checked first, so an existing file (without `force`)
/// aborts before anything is written.
pub fn generate_favicons(src: &RgbaImage, out_dir: &Path, opts: &FaviconOptions) -> Result<Vec<Written>> {
    let ico_sizes = normalize_sizes(&opts.ico_sizes)?;
    let plan = Plan { force: opts.force, dry_run: opts.dry_run, ..Plan::default() };

    if !opts.force {
        let names = PNG_TARGETS.iter().map(|(n, _)| *n).chain([ICO_NAME, MANIFEST_NAME]);
        for name in names {
            let path = out_dir.join(name);
            if path.exists() {
                return Err(Error::AlreadyExists(path));
            }
        }
    }

    let pngs = PNG_TARGETS
        .par_iter()
        .map(|&(name, size)| {
            let bg = if name == APPLE_ICON {
                Some(opts.background.filter(|c| c.0[3] == 255).unwrap_or(Color([255, 255, 255, 255])))
            } else {
                opts.background
            };
            let img = render_square(src, size, opts.fit, opts.padding, bg);
            encode_png(&img).map(|data| (name, size, data))
        })
        .collect::<Result<Vec<_>>>()?;

    let ico_images: Vec<(u32, RgbaImage)> = ico_sizes
        .par_iter()
        .map(|&s| (s, render_square(src, s, opts.fit, opts.padding, opts.background)))
        .collect();
    let ico = encode_ico(&ico_images, Encoding::Auto)?;

    let mut written = Vec::with_capacity(pngs.len() + 2);
    for (name, size, data) in pngs {
        written.push(emit(out_dir.join(name), data, vec![size], &plan)?);
    }
    written.push(emit(out_dir.join(ICO_NAME), ico, ico_sizes, &plan)?);
    written.push(emit(out_dir.join(MANIFEST_NAME), web_manifest(opts).into_bytes(), Vec::new(), &plan)?);
    Ok(written)
}
