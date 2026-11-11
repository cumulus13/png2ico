use image::{Rgba, RgbaImage};
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_png2ico"))
}

fn make_png(dir: &std::path::Path, name: &str, w: u32, h: u32) -> std::path::PathBuf {
    let p = dir.join(name);
    RgbaImage::from_pixel(w, h, Rgba([200, 30, 30, 255])).save(&p).unwrap();
    p
}

#[test]
fn legacy_invocation_single_file() {
    let d = tempfile::tempdir().unwrap();
    let png = make_png(d.path(), "logo.png", 300, 200);
    let out = bin().arg(&png).args(["-S", "-s", "16,32,256"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    let data = std::fs::read(d.path().join("logo.ico")).unwrap();
    let dir = png2ico::read_directory(&data).unwrap();
    assert_eq!(dir.iter().map(|e| e.width).collect::<Vec<_>>(), vec![16, 32, 256]);
}

#[test]
fn per_size_files_and_overwrite_protection() {
    let d = tempfile::tempdir().unwrap();
    let png = make_png(d.path(), "a.png", 64, 64);
    assert!(bin().arg(&png).args(["-s", "16,32"]).status().unwrap().success());
    assert!(d.path().join("a_16.ico").exists() && d.path().join("a_32.ico").exists());
    // second run must refuse to overwrite
    assert!(!bin().arg(&png).args(["-s", "16,32"]).status().unwrap().success());
    // but succeeds with --force
    assert!(bin().arg(&png).args(["-s", "16,32", "--force"]).status().unwrap().success());
}

#[test]
fn dry_run_writes_nothing_and_missing_file_fails() {
    let d = tempfile::tempdir().unwrap();
    let png = make_png(d.path(), "b.png", 32, 32);
    assert!(bin().arg(&png).args(["-S", "-n"]).status().unwrap().success());
    assert!(!d.path().join("b.ico").exists());
    assert!(!bin().arg(d.path().join("nope.png")).status().unwrap().success());
}

#[test]
fn glob_and_directory_inputs() {
    let d = tempfile::tempdir().unwrap();
    make_png(d.path(), "x.png", 32, 32);
    make_png(d.path(), "y.png", 32, 32);
    let pattern = d.path().join("*.png");
    assert!(bin().arg(pattern).args(["-S", "-s", "16"]).status().unwrap().success());
    assert!(d.path().join("x.ico").exists() && d.path().join("y.ico").exists());
}
