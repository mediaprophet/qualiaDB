//! Local private DICOM directory discovery and Part-10 file enumeration helpers.

use std::fs;
use std::path::{Path, PathBuf};

use super::parser::split_dicom_payload;

/// Default relative path for gitignored private DICOM slices (local dev only).
pub const DEFAULT_LOCAL_DICOM_DIR: &str = "app-development/DICOM-20250126T150544Z-001";

/// Resolve private local DICOM root: `QUALIA_DICOM_FIXTURE_DIR` env, then workspace-relative default.
pub fn resolve_local_dicom_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("QUALIA_DICOM_FIXTURE_DIR") {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            return path.canonicalize().ok().or(Some(path));
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    candidates.push(PathBuf::from(DEFAULT_LOCAL_DICOM_DIR));
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        candidates.push(
            PathBuf::from(manifest)
                .join("..")
                .join("..")
                .join(DEFAULT_LOCAL_DICOM_DIR),
        );
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(DEFAULT_LOCAL_DICOM_DIR));
        candidates.push(cwd.join("..").join("..").join(DEFAULT_LOCAL_DICOM_DIR));
    }

    for path in candidates {
        if path.is_dir() {
            return path.canonicalize().ok().or(Some(path));
        }
    }
    None
}

/// Returns true when the byte slice looks like DICOM Part 10 (preamble + `DICM`).
pub fn is_dicom_part10(bytes: &[u8]) -> bool {
    bytes.len() >= 132 && &bytes[128..132] == b"DICM"
}

/// True when the buffer contains extractable Pixel Data for split-ingest.
pub fn dicom_has_pixel_data(data: &[u8]) -> bool {
    split_dicom_payload(data).is_ok()
}

/// Collect up to `max_files` DICOM Part-10 paths under `root` (cold-path helper; may allocate).
pub fn collect_dicom_paths_under(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_dicom_paths_recursive(root, max_files, &mut out);
    out
}

/// Image slices only (Pixel Data present), preferring `IM*` filenames typical of PACS export.
pub fn collect_dicom_image_paths_under(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut all = Vec::new();
    collect_dicom_paths_recursive(root, max_files.saturating_mul(8).max(32), &mut all);
    let mut images: Vec<PathBuf> = all
        .into_iter()
        .filter(|path| {
            fs::read(path)
                .ok()
                .map(|bytes| dicom_has_pixel_data(&bytes))
                .unwrap_or(false)
        })
        .collect();
    images.sort_by_key(|p| {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        (
            !(name.starts_with("IM") || name.starts_with("im")),
            name.to_string(),
        )
    });
    images.truncate(max_files);
    images
}

/// Bounded depth-first walk collecting paths whose bytes pass the
/// `DICM` magic check into `out` (stops at `max_files`).
fn collect_dicom_paths_recursive(dir: &Path, max_files: usize, out: &mut Vec<PathBuf>) {
    if out.len() >= max_files {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if out.len() >= max_files {
            break;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_dicom_paths_recursive(&path, max_files, out);
        } else if path.is_file() {
            if let Ok(bytes) = fs::read(&path) {
                if is_dicom_part10(&bytes) {
                    out.push(path);
                }
            }
        }
    }
}
