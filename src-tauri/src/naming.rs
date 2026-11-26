// SPDX-License-Identifier: GPL-3.0-or-later
//! Output folder, file-name templates and conflict handling.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use crate::options::{ConflictPolicy, OutputOptions};

/// Values that `{tokens}` in the name template are replaced with.
pub struct NameContext<'a> {
    /// Source file name without extension.
    pub stem: &'a str,
    pub width: u32,
    pub height: u32,
    /// Output extension without the dot, e.g. `webp`.
    pub format: &'a str,
    /// `YYYY-MM-DD`, fixed for the whole batch.
    pub date: &'a str,
    /// 1-based position in the batch.
    pub index: usize,
}

pub const DEFAULT_TEMPLATE: &str = "{name}";

/// Characters that are invalid in Windows file names. They're stripped on every OS
/// so output stays portable (USB sticks, shared drives, cloud sync).
const FORBIDDEN: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

const RESERVED_WINDOWS_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

const MAX_STEM_CHARS: usize = 200;

/// Replace `{tokens}` in `template`. Unknown tokens are left as typed.
pub fn render_template(template: &str, ctx: &NameContext) -> String {
    let template = if template.trim().is_empty() {
        DEFAULT_TEMPLATE
    } else {
        template
    };
    template
        .replace("{name}", ctx.stem)
        .replace("{width}", &ctx.width.to_string())
        .replace("{height}", &ctx.height.to_string())
        .replace("{format}", ctx.format)
        .replace("{date}", ctx.date)
        .replace("{index}", &ctx.index.to_string())
}

/// Make `stem` a valid file name (without extension) on Windows, Linux and macOS.
pub fn sanitize_stem(stem: &str, fallback: &str) -> String {
    let cleaned: String = stem
        .chars()
        .map(|c| {
            if FORBIDDEN.contains(&c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let mut cleaned = cleaned
        .trim_start()
        .trim_end_matches(['.', ' '])
        .chars()
        .take(MAX_STEM_CHARS)
        .collect::<String>()
        .trim_end_matches(['.', ' '])
        .to_string();

    if cleaned.is_empty() {
        cleaned = if fallback.is_empty() {
            "image".to_string()
        } else {
            sanitize_stem(fallback, "image")
        };
    }

    // Windows also reserves `CON.txt`, `com1.backup`, ... so break the device name itself.
    let base_len = cleaned.split('.').next().unwrap_or("").len();
    if RESERVED_WINDOWS_NAMES.contains(&cleaned[..base_len].to_ascii_uppercase().as_str()) {
        cleaned.insert(base_len, '_');
    }
    cleaned
}

/// Only keep plain folder names from `rel_dir` so it can't escape the output folder.
fn safe_relative(rel_dir: &str) -> PathBuf {
    Path::new(rel_dir)
        .components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part),
            _ => None,
        })
        .collect()
}

/// Folder the converted file for `src` goes into.
pub fn output_dir(src: &Path, rel_dir: &str, out: &OutputOptions) -> PathBuf {
    match out.directory.as_deref().map(str::trim) {
        Some(dir) if !dir.is_empty() => {
            let base = PathBuf::from(dir);
            if out.keep_structure {
                base.join(safe_relative(rel_dir))
            } else {
                base
            }
        }
        _ => src
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")),
    }
}

/// Key used to compare paths: case-insensitive on Windows (and macOS by default).
fn path_key(p: &Path) -> String {
    let s = p.to_string_lossy().to_string();
    if cfg!(any(windows, target_os = "macos")) {
        s.to_lowercase()
    } else {
        s
    }
}

pub fn is_same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => path_key(&a) == path_key(&b),
        _ => path_key(a) == path_key(b),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Resolution {
    Write(PathBuf),
    Skip(String),
}

/// Hands out unique output paths for one batch, safe to share between threads.
#[derive(Default)]
pub struct NameReserver {
    taken: Mutex<HashSet<String>>,
}

impl NameReserver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decide where to write `desired` for the source file `src`.
    ///
    /// - Two files in the same batch never get the same name (later ones are renamed).
    /// - The source file is never overwritten, whatever the policy.
    /// - `policy` decides what happens with files that already exist on disk.
    pub fn resolve(&self, desired: &Path, src: &Path, policy: ConflictPolicy) -> Resolution {
        let mut taken = self.taken.lock().unwrap_or_else(|e| e.into_inner());

        let in_batch = taken.contains(&path_key(desired));
        let is_source = is_same_file(desired, src);
        let on_disk = desired.exists();

        let chosen = if !in_batch && !is_source && !on_disk {
            desired.to_path_buf()
        } else if !in_batch && !is_source {
            // Only an older file on disk is in the way.
            match policy {
                ConflictPolicy::Overwrite => desired.to_path_buf(),
                ConflictPolicy::Skip => {
                    return Resolution::Skip("Output file already exists".to_string())
                }
                ConflictPolicy::Rename => next_free(desired, &taken),
            }
        } else {
            next_free(desired, &taken)
        };

        taken.insert(path_key(&chosen));
        Resolution::Write(chosen)
    }
}

/// `photo.jpg` → `photo (1).jpg`, `photo (2).jpg`, ... (first free one).
fn next_free(desired: &Path, taken: &HashSet<String>) -> PathBuf {
    let dir = desired.parent().unwrap_or_else(|| Path::new(""));
    let stem = desired
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = desired
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    (1u32..)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists() && !taken.contains(&path_key(p)))
        .expect("ran out of u32 suffixes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn ctx() -> NameContext<'static> {
        NameContext {
            stem: "holiday",
            width: 1920,
            height: 1080,
            format: "webp",
            date: "2026-11-24",
            index: 7,
        }
    }

    #[test]
    fn renders_all_tokens() {
        let t = "{name}_{width}x{height}_{format}_{date}_{index}";
        assert_eq!(
            render_template(t, &ctx()),
            "holiday_1920x1080_webp_2026-11-24_7"
        );
    }

    #[test]
    fn empty_template_uses_name() {
        assert_eq!(render_template("  ", &ctx()), "holiday");
        assert_eq!(render_template("{nope}", &ctx()), "{nope}");
    }

    #[test]
    fn sanitizes_forbidden_and_reserved() {
        assert_eq!(
            sanitize_stem("a/b\\c:d*e?f\"g<h>i|j", "x"),
            "a_b_c_d_e_f_g_h_i_j"
        );
        assert_eq!(sanitize_stem("  trailing dots... ", "x"), "trailing dots");
        assert_eq!(sanitize_stem("con", "x"), "con_");
        assert_eq!(sanitize_stem("COM1.backup", "x"), "COM1_.backup");
        assert_eq!(sanitize_stem("", "fallback"), "fallback");
        assert_eq!(sanitize_stem("...", ""), "image");
        assert_eq!(sanitize_stem("tab\there", "x"), "tab_here");
        assert_eq!(sanitize_stem(&"a".repeat(500), "x").len(), MAX_STEM_CHARS);
    }

    fn out(directory: Option<&str>, keep_structure: bool) -> OutputOptions {
        OutputOptions {
            directory: directory.map(String::from),
            keep_structure,
            name_template: DEFAULT_TEMPLATE.into(),
            conflict: ConflictPolicy::Rename,
        }
    }

    #[test]
    fn output_dir_variants() {
        let src = Path::new("photos").join("trip").join("a.jpg");
        assert_eq!(
            output_dir(&src, "trip", &out(None, true)),
            Path::new("photos").join("trip")
        );
        assert_eq!(
            output_dir(&src, "trip", &out(Some("out"), true)),
            Path::new("out").join("trip")
        );
        assert_eq!(
            output_dir(&src, "trip", &out(Some("out"), false)),
            PathBuf::from("out")
        );
        // A hostile rel_dir can't climb out of the output folder.
        let escape = Path::new("..").join("..").join("etc");
        assert_eq!(
            output_dir(&src, &escape.to_string_lossy(), &out(Some("out"), true)),
            Path::new("out").join("etc")
        );
    }

    #[test]
    fn rename_policy_and_batch_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.png");
        fs::write(&src, b"x").unwrap();
        let existing = dir.path().join("a.webp");
        fs::write(&existing, b"x").unwrap();

        let r = NameReserver::new();
        // Exists on disk → renamed.
        assert_eq!(
            r.resolve(&existing, &src, ConflictPolicy::Rename),
            Resolution::Write(dir.path().join("a (1).webp"))
        );
        // Same desired name again in this batch → next number.
        assert_eq!(
            r.resolve(&existing, &src, ConflictPolicy::Rename),
            Resolution::Write(dir.path().join("a (2).webp"))
        );
        // Fresh name → used as is.
        let fresh = dir.path().join("b.webp");
        assert_eq!(
            r.resolve(&fresh, &src, ConflictPolicy::Rename),
            Resolution::Write(fresh.clone())
        );
        // Batch duplicate is renamed even under Skip/Overwrite.
        assert_eq!(
            r.resolve(&fresh, &src, ConflictPolicy::Skip),
            Resolution::Write(dir.path().join("b (1).webp"))
        );
        assert_eq!(
            r.resolve(&fresh, &src, ConflictPolicy::Overwrite),
            Resolution::Write(dir.path().join("b (2).webp"))
        );
    }

    #[test]
    fn skip_and_overwrite_policies() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.png");
        fs::write(&src, b"x").unwrap();
        let existing = dir.path().join("a.webp");
        fs::write(&existing, b"x").unwrap();

        assert!(matches!(
            NameReserver::new().resolve(&existing, &src, ConflictPolicy::Skip),
            Resolution::Skip(_)
        ));
        assert_eq!(
            NameReserver::new().resolve(&existing, &src, ConflictPolicy::Overwrite),
            Resolution::Write(existing.clone())
        );
    }

    #[test]
    fn never_overwrites_the_source() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("photo.png");
        fs::write(&src, b"x").unwrap();

        for policy in [
            ConflictPolicy::Overwrite,
            ConflictPolicy::Skip,
            ConflictPolicy::Rename,
        ] {
            assert_eq!(
                NameReserver::new().resolve(&src, &src, policy),
                Resolution::Write(dir.path().join("photo (1).png")),
                "{policy:?}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_paths_compare_case_insensitively() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("Photo.PNG");
        fs::write(&src, b"x").unwrap();
        assert_eq!(
            NameReserver::new().resolve(
                &dir.path().join("photo.png"),
                &src,
                ConflictPolicy::Overwrite
            ),
            Resolution::Write(dir.path().join("photo (1).png"))
        );
    }
}
