//! `png2ico favicon` subcommand.

use anyhow::Result;
use clap::{Args, ValueEnum};
use png2ico::favicon::{generate_favicons, html_snippet, FaviconOptions, DEFAULT_ICO_SIZES};
use png2ico::{load_image, Color, Fit};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Clone, Copy, ValueEnum)]
enum FitChoice {
    /// Keep aspect ratio, pad with transparency
    Contain,
    /// Keep aspect ratio, fill and center-crop
    Cover,
    /// Stretch to a square
    Stretch,
}

/// Generate a complete favicon package (favicon.io style): apple-touch-icon,
/// favicon 16/32, favicon.ico, android-chrome 192/512 and site.webmanifest.
#[derive(Args)]
pub struct FaviconArgs {
    /// Source image (PNG, JPEG, WebP, BMP, GIF, TIFF); a square 512px+ image is best
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output directory
    #[arg(short = 'd', long, default_value = "favicon")]
    out_dir: PathBuf,

    /// Web app name for site.webmanifest
    #[arg(long, default_value = "")]
    name: String,

    /// Short web app name (default: same as --name)
    #[arg(long)]
    short_name: Option<String>,

    /// theme_color in site.webmanifest
    #[arg(long, default_value = "#ffffff")]
    theme_color: String,

    /// background_color in site.webmanifest
    #[arg(long, default_value = "#ffffff")]
    background_color: String,

    /// Sizes stored in favicon.ico (comma separated, max 256)
    #[arg(long, value_delimiter = ',', default_values_t = DEFAULT_ICO_SIZES.to_vec())]
    ico_sizes: Vec<u32>,

    /// How to fit non-square images
    #[arg(long, value_enum, default_value_t = FitChoice::Contain)]
    fit: FitChoice,

    /// Margin per side in percent of icon size (0-45)
    #[arg(long, default_value_t = 0.0, value_parser = parse_padding)]
    padding: f32,

    /// Background for padding: #RGB, #RRGGBB, #RRGGBBAA, white, black, transparent
    #[arg(long)]
    background: Option<Color>,

    /// Overwrite existing files
    #[arg(short, long)]
    force: bool,

    /// Show what would be written without writing
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Do not print the HTML <link> snippet
    #[arg(long)]
    no_html: bool,
}

fn parse_padding(s: &str) -> Result<f32, String> {
    match s.parse::<f32>() {
        Ok(v) if (0.0..=45.0).contains(&v) => Ok(v),
        _ => Err("padding must be a number between 0 and 45".to_string()),
    }
}

fn human(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    }
}

pub fn run(a: FaviconArgs, quiet: bool) -> Result<ExitCode> {
    let src = load_image(&a.input)?;
    if src.width().min(src.height()) < 512 {
        tracing::warn!(
            "source is {}x{}; android-chrome-512x512.png will be upscaled (use a 512px+ image for best quality)",
            src.width(),
            src.height()
        );
    }

    let short_name = a.short_name.clone().unwrap_or_else(|| a.name.clone());
    let opts = FaviconOptions {
        fit: match a.fit {
            FitChoice::Contain => Fit::Contain,
            FitChoice::Cover => Fit::Cover,
            FitChoice::Stretch => Fit::Stretch,
        },
        padding: a.padding,
        background: a.background,
        name: a.name,
        short_name,
        theme_color: a.theme_color,
        background_color: a.background_color,
        ico_sizes: a.ico_sizes,
        force: a.force,
        dry_run: a.dry_run,
    };

    let written = generate_favicons(&src, &a.out_dir, &opts)?;

    if !quiet {
        let verb = if a.dry_run { "would write" } else { "✓" };
        println!("{}", a.out_dir.display());
        for w in &written {
            println!("  {verb} {} ({})", w.path.file_name().unwrap_or_default().to_string_lossy(), human(w.bytes));
        }
        if !a.no_html {
            println!("\nAdd to your <head>:\n\n{}", html_snippet());
        }
    }
    Ok(ExitCode::SUCCESS)
}
