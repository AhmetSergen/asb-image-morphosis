// SPDX-License-Identifier: GPL-3.0-or-later
//! Types shared with the frontend (mirrored in `src/lib/types.ts`).

use serde::{Deserialize, Serialize};

/// Target file format chosen by the user.
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Keep the source format (falls back to PNG for read-only formats).
    Original,
    Jpeg,
    Png,
    Webp,
    Avif,
    Gif,
    Bmp,
    Tiff,
    Ico,
}

/// How images are resized. Dimensions of `0` mean "unconstrained".
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum ResizeMode {
    /// Keep original dimensions.
    None,
    /// Scale both sides by a percentage.
    Percent { percent: f64 },
    /// Shrink/grow to fit inside a box, keeping aspect ratio.
    Fit { width: u32, height: u32 },
    /// Set the width, height follows the aspect ratio.
    Width { width: u32 },
    /// Set the height, width follows the aspect ratio.
    Height { height: u32 },
    /// Cover the box and crop the overflow (centred). Output is exactly the box.
    Fill { width: u32, height: u32 },
    /// Force exact dimensions, ignoring aspect ratio.
    Stretch { width: u32, height: u32 },
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PngCompression {
    Fast,
    #[default]
    Default,
    Best,
}

/// What to do when the output file already exists on disk.
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConflictPolicy {
    /// Write `name (1).ext`, `name (2).ext`, ...
    #[default]
    Rename,
    Overwrite,
    Skip,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OutputOptions {
    /// `None` = write next to each source file.
    pub directory: Option<String>,
    /// Re-create the source sub-folder structure inside `directory`.
    pub keep_structure: bool,
    /// File name template, e.g. `{name}_{width}x{height}`.
    pub name_template: String,
    pub conflict: ConflictPolicy,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConvertOptions {
    pub format: OutputFormat,
    /// 1..=100, used by JPEG, lossy WebP and AVIF.
    pub quality: u8,
    pub webp_lossless: bool,
    pub png_compression: PngCompression,
    /// `#rrggbb`, used to flatten transparency for formats without alpha (JPEG).
    pub background: String,
    pub resize: ResizeMode,
    /// Never make an image larger than its source.
    pub no_upscale: bool,
    pub output: OutputOptions,
}

/// One file handed to `convert` by the frontend.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConvertItem {
    pub path: String,
    /// Sub-folder relative to the folder the user added ("" for loose files).
    pub rel_dir: String,
}

/// An image found by `scan_paths`.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImageEntry {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub rel_dir: String,
    pub size_bytes: u64,
    /// Dimensions after applying EXIF orientation.
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SkippedFile {
    pub path: String,
    pub reason: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub entries: Vec<ImageEntry>,
    /// Files the user explicitly added (or known image types) that can't be read.
    pub skipped: Vec<SkippedFile>,
    /// Non-image files silently ignored while walking folders.
    pub ignored: usize,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub converted: usize,
    pub skipped: usize,
    pub failed: usize,
    pub cancelled: bool,
    /// Total size of the sources that were converted.
    pub in_bytes: u64,
    /// Total size of the files written.
    pub out_bytes: u64,
    pub elapsed_ms: u64,
    /// Folders that received at least one file (for "Open output folder").
    pub output_dirs: Vec<String>,
}

/// Streamed to the frontend through a `tauri::ipc::Channel`.
#[derive(Serialize, Clone, Debug)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
pub enum ProgressEvent {
    Started {
        total: usize,
    },
    /// A worker picked up this file.
    FileStarted {
        path: String,
    },
    FileDone {
        path: String,
        output_path: String,
        out_bytes: u64,
        width: u32,
        height: u32,
    },
    FileSkipped {
        path: String,
        reason: String,
    },
    FileFailed {
        path: String,
        error: String,
    },
    Finished {
        summary: Summary,
    },
}

/// Result of `preview_output` for the live preview in the settings panel.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OutputPreview {
    pub path: String,
    pub width: u32,
    pub height: u32,
    /// The target already exists on disk (so the conflict policy will apply).
    pub exists: bool,
    /// The target *is* the source file (it will never be overwritten).
    pub is_source: bool,
}
