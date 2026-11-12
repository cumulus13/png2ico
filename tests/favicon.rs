use image::{Rgba, RgbaImage};
use png2ico::favicon::{generate_favicons, FaviconOptions, PNG_TARGETS};
use png2ico::{read_directory, Error};

#[test]
fn generates_full_package() {
    let d = tempfile::tempdir().unwrap();
    let src = RgbaImage::from_pixel(600, 400, Rgba([20, 120, 220, 255]));
    let opts = FaviconOptions { name: "My \"App\"".into(), short_name: "App".into(), ..FaviconOptions::default() };

    let written = generate_favicons(&src, d.path(), &opts).unwrap();
    assert_eq!(written.len(), 7);

    for (name, size) in PNG_TARGETS {
        let img = image::open(d.path().join(name)).unwrap().to_rgba8();
        assert_eq!(img.dimensions(), (*size, *size), "{name}");
    }

    // apple-touch-icon must be fully opaque (padding is filled with white).
    let apple = image::open(d.path().join("apple-touch-icon.png")).unwrap().to_rgba8();
    assert!(apple.pixels().all(|p| p[3] == 255));

    let ico = std::fs::read(d.path().join("favicon.ico")).unwrap();
    let sizes: Vec<u32> = read_directory(&ico).unwrap().iter().map(|e| e.width).collect();
    assert_eq!(sizes, vec![16, 32, 48]);

    let manifest = std::fs::read_to_string(d.path().join("site.webmanifest")).unwrap();
    assert!(manifest.contains("\"name\":\"My \\\"App\\\"\""));
    assert!(manifest.contains("android-chrome-512x512.png"));
}

#[test]
fn refuses_to_overwrite_without_force() {
    let d = tempfile::tempdir().unwrap();
    let src = RgbaImage::from_pixel(64, 64, Rgba([1, 2, 3, 255]));
    let opts = FaviconOptions::default();
    generate_favicons(&src, d.path(), &opts).unwrap();
    assert!(matches!(generate_favicons(&src, d.path(), &opts), Err(Error::AlreadyExists(_))));
    let forced = FaviconOptions { force: true, ..FaviconOptions::default() };
    assert!(generate_favicons(&src, d.path(), &forced).is_ok());
}

#[test]
fn dry_run_writes_nothing() {
    let d = tempfile::tempdir().unwrap();
    let out = d.path().join("fav");
    let src = RgbaImage::from_pixel(64, 64, Rgba([1, 2, 3, 255]));
    let opts = FaviconOptions { dry_run: true, ..FaviconOptions::default() };
    assert_eq!(generate_favicons(&src, &out, &opts).unwrap().len(), 7);
    assert!(!out.exists());
}
