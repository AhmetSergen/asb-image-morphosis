// SPDX-License-Identifier: GPL-3.0-or-later
//! Turn files/folders the user picked or dropped into a list of readable images.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use image::metadata::Orientation;
use image::{ImageDecoder, ImageReader};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::options::{ImageEntry, ScanResult, SkippedFile};

/// Extensions we can decode (lower case).
pub const READABLE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "jpe", "jfif", "png", "gif", "bmp", "dib", "tif", "tiff", "webp", "ico", "tga",
    "qoi", "ppm", "pgm", "pbm", "pnm", "pam", "hdr", "exr",
];

/// Image types people commonly have that we know we can't read yet.
const KNOWN_UNSUPPORTED: &[(&str, &str)] = &[
    ("avif", "AVIF input isn't supported yet (AVIF output is)"),
    ("heic", "HEIC/HEIF isn't supported yet"),
    ("heif", "HEIC/HEIF isn't supported yet"),
    ("jxl", "JPEG XL isn't supported yet"),
    ("svg", "SVG is a vector format and isn't supported"),
    ("psd", "Photoshop files aren't supported"),
    ("raw", "Camera RAW files aren't supported"),
    ("cr2", "Camera RAW files aren't supported"),
    ("cr3", "Camera RAW files aren't supported"),
    ("nef", "Camera RAW files aren't supported"),
    ("arw", "Camera RAW files aren't supported"),
    ("dng", "Camera RAW files aren't supported"),
    ("orf", "Camera RAW files aren't supported"),
    ("rw2", "Camera RAW files aren't supported"),
];

/// Lower-case extension of `path`, or "" if it has none.
pub fn ext_of(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

pub fn is_readable(path: &Path) -> bool {
    READABLE_EXTENSIONS.contains(&ext_of(path).as_str())
}

fn unsupported_reason(path: &Path) -> Option<&'static str> {
    let ext = ext_of(path);
    KNOWN_UNSUPPORTED
        .iter()
        .find(|(e, _)| *e == ext)
        .map(|(_, reason)| *reason)
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .map(|n| n.to_string_lossy().starts_with('.'))
        .unwrap_or(false)
}

/// Does this orientation swap width and height?
pub fn swaps_axes(o: Orientation) -> bool {
    matches!(
        o,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    )
}

/// Read the image header only: `(width, height)` with EXIF orientation applied.
pub fn probe(path: &Path) -> Result<(u32, u32), String> {
    let reader = ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let (w, h) = decoder.dimensions();
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    Ok(if swaps_axes(orientation) {
        (h, w)
    } else {
        (w, h)
    })
}

/// A candidate file plus the folder it should be considered relative to.
struct Candidate {
    path: PathBuf,
    root: PathBuf,
}

fn rel_dir(path: &Path, root: &Path) -> String {
    path.parent()
        .and_then(|p| p.strip_prefix(root).ok())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn dedupe_key(p: &Path) -> String {
    let p = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let s = p.to_string_lossy().to_string();
    if cfg!(any(windows, target_os = "macos")) {
        s.to_lowercase()
    } else {
        s
    }
}

/// Expand `paths` (files and/or folders) into readable images.
///
/// Folders are walked (recursively if `recursive`), skipping hidden files and folders.
/// Files that were picked explicitly but can't be read are reported in `skipped`.
pub fn scan(paths: &[String], recursive: bool) -> ScanResult {
    let mut result = ScanResult::default();
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    // Files listed twice (or inside an added folder too) are only kept once.
    let mut push = |path: PathBuf, root: PathBuf| {
        if seen.insert(dedupe_key(&path)) {
            candidates.push(Candidate { path, root });
        }
    };

    for raw in paths {
        let path = PathBuf::from(raw);
        if path.is_dir() {
            let walker = WalkDir::new(&path)
                .max_depth(if recursive { usize::MAX } else { 1 })
                .sort_by_file_name()
                .into_iter()
                .filter_entry(|e| e.depth() == 0 || !is_hidden(e.path()));
            for entry in walker.flatten() {
                if !entry.file_type().is_file() {
                    continue;
                }
                let p = entry.into_path();
                if is_readable(&p) {
                    push(p, path.clone());
                } else if let Some(reason) = unsupported_reason(&p) {
                    result.skipped.push(SkippedFile {
                        path: p.to_string_lossy().to_string(),
                        reason: reason.to_string(),
                    });
                } else {
                    result.ignored += 1;
                }
            }
        } else if path.is_file() {
            if is_readable(&path) {
                let root = path.parent().map(Path::to_path_buf).unwrap_or_default();
                push(path, root);
            } else {
                result.skipped.push(SkippedFile {
                    reason: unsupported_reason(&path)
                        .unwrap_or("Not a supported image type")
                        .to_string(),
                    path: raw.clone(),
                });
            }
        } else {
            result.skipped.push(SkippedFile {
                path: raw.clone(),
                reason: "File or folder not found".to_string(),
            });
        }
    }

    // Header probing touches the disk; do it in parallel (network drives, big folders).
    let probed: Vec<Result<ImageEntry, SkippedFile>> = candidates
        .par_iter()
        .map(|c| {
            let path_str = c.path.to_string_lossy().to_string();
            match probe(&c.path) {
                Ok((width, height)) => Ok(ImageEntry {
                    name: c
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    ext: ext_of(&c.path),
                    rel_dir: rel_dir(&c.path, &c.root),
                    size_bytes: std::fs::metadata(&c.path).map(|m| m.len()).unwrap_or(0),
                    width,
                    height,
                    path: path_str,
                }),
                Err(e) => Err(SkippedFile {
                    path: path_str,
                    reason: format!("Can't read image: {e}"),
                }),
            }
        })
        .collect();

    for p in probed {
        match p {
            Ok(entry) => result.entries.push(entry),
            Err(skip) => result.skipped.push(skip),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use std::fs;

    fn write_png(path: &Path, w: u32, h: u32) {
        RgbImage::from_pixel(w, h, Rgb([10, 20, 30]))
            .save(path)
            .unwrap();
    }

    #[test]
    fn extensions_are_case_insensitive() {
        assert!(is_readable(Path::new("a/B.JPG")));
        assert!(is_readable(Path::new("photo.Png")));
        assert!(!is_readable(Path::new("notes.txt")));
        assert!(!is_readable(Path::new("no_extension")));
    }

    #[test]
    fn scans_folders_recursively_skipping_hidden() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("sub").join("deeper")).unwrap();
        fs::create_dir_all(root.join(".hidden")).unwrap();
        write_png(&root.join("a.png"), 4, 3);
        write_png(&root.join("sub").join("B.PNG"), 2, 2);
        write_png(&root.join("sub").join("deeper").join("c.png"), 1, 1);
        write_png(&root.join(".hidden").join("secret.png"), 1, 1);
        write_png(&root.join(".dotfile.png"), 1, 1);
        fs::write(root.join("readme.txt"), "hi").unwrap();
        fs::write(root.join("phone.heic"), "not really").unwrap();
        fs::write(root.join("broken.png"), "not a png").unwrap();

        let r = scan(&[root.to_string_lossy().to_string()], true);
        let mut names: Vec<_> = r.entries.iter().map(|e| e.name.as_str()).collect();
        names.sort();
        assert_eq!(names, ["B.PNG", "a.png", "c.png"]);
        assert_eq!(r.ignored, 1); // readme.txt
        assert_eq!(r.skipped.len(), 2); // heic + broken png

        let deep = r.entries.iter().find(|e| e.name == "c.png").unwrap();
        assert_eq!(
            PathBuf::from(&deep.rel_dir),
            Path::new("sub").join("deeper")
        );
        let a = r.entries.iter().find(|e| e.name == "a.png").unwrap();
        assert_eq!((a.width, a.height, a.rel_dir.as_str()), (4, 3, ""));

        let flat = scan(&[root.to_string_lossy().to_string()], false);
        assert_eq!(flat.entries.len(), 1);
    }

    #[test]
    fn dedupes_and_reports_explicit_files() {
        let dir = tempfile::tempdir().unwrap();
        let png = dir.path().join("x.png");
        write_png(&png, 2, 2);
        let txt = dir.path().join("x.txt");
        fs::write(&txt, "hi").unwrap();
        let p = png.to_string_lossy().to_string();

        let r = scan(
            &[
                p.clone(),
                p,
                dir.path().to_string_lossy().to_string(),
                txt.to_string_lossy().to_string(),
                dir.path().join("missing.png").to_string_lossy().to_string(),
            ],
            false,
        );
        assert_eq!(r.entries.len(), 1);
        assert_eq!(r.skipped.len(), 2); // x.txt (explicit) + missing
    }
}
