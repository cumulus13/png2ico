//! # png2ico
//!
//! Convert raster images (PNG, JPEG, WebP, BMP, GIF, TIFF) to Windows `.ico`
//! files with multiple sizes, alpha-correct resampling and a hand-written,
//! standards-compliant ICO encoder.
//!
//! ```no_run
//! use png2ico::{convert_file, Layout, Options, Plan};
//! use std::path::Path;
//!
//! let opts = Options { sizes: vec![16, 32, 48, 256], ..Options::default() };
//! let plan = Plan { layout: Layout::Single, force: true, ..Plan::default() };
//! let written = convert_file(Path::new("logo.png"), &opts, &plan)?;
//! println!("wrote {}", written[0].path.display());
//! # Ok::<(), png2ico::Error>(())
//! ```

pub mod color;
pub mod convert;
pub mod error;
pub mod ico;
pub mod imaging;
pub mod inputs;

pub use color::Color;
pub use convert::{
    convert_file, load_image, normalize_sizes, render_all, Layout, Options, Plan, Written,
    SIZES_ALL, SIZES_FAVICON, SIZES_WINDOWS,
};
pub use error::{Error, Result};
pub use ico::{encode_ico, read_directory, Encoding, EntryFormat, IcoEntry, MAX_ICO_SIZE};
pub use imaging::Fit;
