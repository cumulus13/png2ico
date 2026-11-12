//! Error types for the `png2ico` library.

use std::path::PathBuf;
use thiserror::Error;

/// Convenient result alias.
pub type Result<T> = std::result::Result<T, Error>;

/// All errors that can be produced by this crate.
#[derive(Debug, Error)]
pub enum Error {
    #[error("file not found: {}", .0.display())]
    NotFound(PathBuf),

    #[error("invalid size {0}: must be between 1 and 256")]
    InvalidSize(u32),

    #[error("no valid sizes to generate")]
    NoSizes,

    #[error("output already exists (use --force to overwrite): {}", .0.display())]
    AlreadyExists(PathBuf),

    #[error(
        "invalid color `{0}`: expected #RGB, #RGBA, #RRGGBB, #RRGGBBAA or a name (transparent, white, black)"
    )]
    InvalidColor(String),

    #[error("invalid ICO file: {0}")]
    InvalidIco(String),

    #[error("{}: {source}", .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    #[error("I/O error: {0}")]
    BareIo(#[from] std::io::Error),
}

impl Error {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io { path: path.into(), source }
    }
}
