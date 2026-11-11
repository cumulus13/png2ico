//! Resolving CLI input arguments (files, directories, glob patterns).

use crate::convert::SUPPORTED_EXTENSIONS;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Result of expanding input arguments.
#[derive(Debug, Default)]
pub struct Collected {
    pub files: Vec<PathBuf>,
    /// Arguments that matched nothing.
    pub missing: Vec<String>,
}

fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| SUPPORTED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

fn scan_dir(dir: &Path, recursive: bool, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            if recursive {
                scan_dir(&p, recursive, out);
            }
        } else if p.is_file() && is_supported(&p) {
            out.push(p);
        }
    }
}

/// Expand files, directories and glob patterns (needed on Windows shells,
/// which don't expand `*.png` themselves). Duplicates are removed, order kept.
pub fn collect_inputs(args: &[String], recursive: bool) -> Collected {
    let mut result = Collected::default();
    let mut seen = HashSet::new();
    let mut push = |p: PathBuf, files: &mut Vec<PathBuf>| {
        let key = fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
        if seen.insert(key) {
            files.push(p);
        }
    };

    for arg in args {
        let path = PathBuf::from(arg);
        if path.is_file() {
            push(path, &mut result.files);
        } else if path.is_dir() {
            let mut found = Vec::new();
            scan_dir(&path, recursive, &mut found);
            if found.is_empty() {
                result.missing.push(format!("{arg} (no images found)"));
            }
            for f in found {
                push(f, &mut result.files);
            }
        } else if arg.contains(['*', '?', '[']) {
            let mut matched = false;
            if let Ok(paths) = glob::glob(arg) {
                for p in paths.flatten().filter(|p| p.is_file()) {
                    matched = true;
                    push(p, &mut result.files);
                }
            }
            if !matched {
                result.missing.push(arg.clone());
            }
        } else {
            result.missing.push(arg.clone());
        }
    }
    result
}
