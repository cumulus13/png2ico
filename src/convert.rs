//! High-level conversion pipeline: load -> render sizes -> encode -> write.

use crate::color::Color;
use crate::error::{Error, Result};
use crate::ico::{encode_ico, Encoding, MAX_ICO_SIZE};
use crate::imaging::{render_square, Fit};
use image::RgbaImage;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Sizes bundled with the original script (minus 512, which ICO cannot store).
pub const SIZES_ALL: &[u32] = &[16, 20, 24, 32, 40, 48, 64, 72, 96, 128, 256];
/// Typical Windows application icon set.
pub const SIZES_WINDOWS: &[u32] = &[16, 24, 32, 48, 64, 128, 256];
/// Classic favicon set.
pub const SIZES_FAVICON: &[u32] = &[16, 32, 48];

/// Image extensions picked up when scanning directories.
pub const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff"];

/// Rendering/encoding options.
#[derive(Debug, Clone)]
pub struct Options {
    pub sizes: Vec<u32>,
    pub fit: Fit,
    /// Margin per side, percent of icon size (0..=45).
    pub padding: f32,
    pub background: Option<Color>,
    pub encoding: Encoding,
    /// When `false`, sizes larger than the source image are skipped.
    pub allow_upscale: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            sizes: SIZES_ALL.to_vec(),
            fit: Fit::Contain,
            padding: 0.0,
            background: None,
            encoding: Encoding::Auto,
            allow_upscale: true,
        }
    }
}

/// Output layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// One `name_<size>.ico` per size.
    #[default]
    PerSize,
    /// A single multi-size `name.ico`.
    Single,
}

/// Where and how to write results.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub layout: Layout,
    pub out_dir: Option<PathBuf>,
    /// Explicit output file (only meaningful with [`Layout::Single`]).
    pub output: Option<PathBuf>,
    pub force: bool,
    pub dry_run: bool,
}

/// A file that was (or, in dry-run mode, would be) written.
#[derive(Debug, Clone)]
pub struct Written {
    pub path: PathBuf,
    pub bytes: u64,
    pub sizes: Vec<u32>,
}

/// Validate, deduplicate and sort sizes. Sizes above 256 are skipped with a warning.
pub fn normalize_sizes(sizes: &[u32]) -> Result<Vec<u32>> {
    let mut out = Vec::with_capacity(sizes.len());
    for &s in sizes {
        if s == 0 {
            return Err(Error::InvalidSize(s));
        }
        if s > MAX_ICO_SIZE {
            warn!("size {s} exceeds the ICO limit of {MAX_ICO_SIZE}px and was skipped");
            continue;
        }
        out.push(s);
    }
    out.sort_unstable();
    out.dedup();
    if out.is_empty() {
        return Err(Error::NoSizes);
    }
    Ok(out)
}

/// Load any supported raster image as RGBA8, sniffing the format from content.
pub fn load_image(path: &Path) -> Result<RgbaImage> {
    if !path.is_file() {
        return Err(Error::NotFound(path.to_path_buf()));
    }
    let reader = image::ImageReader::open(path)
        .map_err(|e| Error::io(path, e))?
        .with_guessed_format()
        .map_err(|e| Error::io(path, e))?;
    Ok(reader.decode()?.to_rgba8())
}

/// Render all requested sizes (in parallel) from a decoded image.
pub fn render_all(src: &RgbaImage, opts: &Options) -> Result<Vec<(u32, RgbaImage)>> {
    let mut sizes = normalize_sizes(&opts.sizes)?;
    let longest = src.width().max(src.height());

    if !opts.allow_upscale {
        let filtered: Vec<u32> = sizes.iter().copied().filter(|&s| s <= longest).collect();
        if filtered.is_empty() {
            warn!("source ({}x{}) is smaller than every requested size; upscaling anyway", src.width(), src.height());
        } else {
            sizes = filtered;
        }
    } else if sizes.iter().any(|&s| s > longest) {
        debug!("source is {longest}px; larger sizes will be upscaled");
    }

    Ok(sizes
        .par_iter()
        .map(|&s| (s, render_square(src, s, opts.fit, opts.padding, opts.background)))
        .collect())
}

fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, data).map_err(|e| Error::io(&tmp, e))?;
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(Error::io(path, e));
    }
    Ok(())
}

fn emit(path: PathBuf, data: Vec<u8>, sizes: Vec<u32>, plan: &Plan) -> Result<Written> {
    if path.exists() && !plan.force {
        return Err(Error::AlreadyExists(path));
    }
    let bytes = data.len() as u64;
    if plan.dry_run {
        debug!("dry-run: would write {}", path.display());
    } else {
        write_atomic(&path, &data)?;
    }
    Ok(Written { path, bytes, sizes })
}

/// Convert one image file according to `opts` and `plan`.
pub fn convert_file(input: &Path, opts: &Options, plan: &Plan) -> Result<Vec<Written>> {
    let src = load_image(input)?;
    debug!("loaded {} ({}x{})", input.display(), src.width(), src.height());
    let rendered = render_all(&src, opts)?;

    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "icon".to_string());
    let dir = plan
        .out_dir
        .clone()
        .or_else(|| input.parent().map(Path::to_path_buf))
        .unwrap_or_default();

    match plan.layout {
        Layout::Single => {
            let path = plan.output.clone().unwrap_or_else(|| dir.join(format!("{stem}.ico")));
            let sizes = rendered.iter().map(|(s, _)| *s).collect();
            let data = encode_ico(&rendered, opts.encoding)?;
            Ok(vec![emit(path, data, sizes, plan)?])
        }
        Layout::PerSize => {
            let mut written = Vec::with_capacity(rendered.len());
            for pair in &rendered {
                let size = pair.0;
                let data = encode_ico(std::slice::from_ref(pair), opts.encoding)?;
                written.push(emit(dir.join(format!("{stem}_{size}.ico")), data, vec![size], plan)?);
            }
            Ok(written)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_sorts_dedups_and_drops_oversize() {
        assert_eq!(normalize_sizes(&[64, 16, 16, 512, 32]).unwrap(), vec![16, 32, 64]);
        assert!(matches!(normalize_sizes(&[0]), Err(Error::InvalidSize(0))));
        assert!(matches!(normalize_sizes(&[512, 1024]), Err(Error::NoSizes)));
        assert!(matches!(normalize_sizes(&[]), Err(Error::NoSizes)));
    }
}
