// SPDX-License-Identifier: GPL-3.0-or-later
//! Commands the frontend calls through `invoke()`.

use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::Ordering;

use base64::Engine;
use image::codecs::jpeg::JpegEncoder;
use image::ImageFormat;
use tauri::ipc::Channel;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::options::{
    ConvertItem, ConvertOptions, OutputPreview, ProgressEvent, ScanResult, Summary,
};
use crate::{pipeline, scan, AppState};

/// Run blocking work off the async runtime and flatten the join error.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("Background task failed: {e}"))?
}

#[tauri::command]
pub async fn scan_paths(paths: Vec<String>, recursive: bool) -> Result<ScanResult, String> {
    blocking(move || Ok(scan::scan(&paths, recursive))).await
}

/// Small preview as a `data:` URL for the file list.
#[tauri::command]
pub async fn thumbnail(path: String, size: u32) -> Result<String, String> {
    blocking(move || {
        let img = pipeline::load(Path::new(&path)).map_err(|e| e.to_string())?;
        let size = size.clamp(16, 512);
        let thumb = img.thumbnail(size, size);
        let mut bytes = Vec::new();
        let mime = if thumb.color().has_alpha() {
            thumb
                .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
                .map_err(|e| e.to_string())?;
            "image/png"
        } else {
            image::DynamicImage::ImageRgb8(thumb.to_rgb8())
                .write_with_encoder(JpegEncoder::new_with_quality(&mut bytes, 80))
                .map_err(|e| e.to_string())?;
            "image/jpeg"
        };
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(format!("data:{mime};base64,{b64}"))
    })
    .await
}

/// Where the first file would be written, for the live preview.
#[tauri::command]
pub async fn preview_output(
    item: ConvertItem,
    options: ConvertOptions,
) -> Result<OutputPreview, String> {
    blocking(move || {
        pipeline::Batch::new(&options)
            .preview(&item)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Resets the "running" flag when the conversion ends, even on panic.
struct RunningGuard<'a>(&'a std::sync::atomic::AtomicBool);
impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

#[tauri::command]
pub async fn convert(
    state: State<'_, AppState>,
    files: Vec<ConvertItem>,
    options: ConvertOptions,
    on_progress: Channel<ProgressEvent>,
) -> Result<Summary, String> {
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("A conversion is already running".to_string());
    }
    state.cancel.store(false, Ordering::SeqCst);

    let running = state.running.clone();
    let cancel = state.cancel.clone();
    blocking(move || {
        let _guard = RunningGuard(&running);
        Ok(pipeline::run_batch(files, &options, &cancel, |event| {
            // The window may be gone; nothing useful to do with the error.
            let _ = on_progress.send(event);
        }))
    })
    .await
}

#[tauri::command]
pub fn cancel_convert(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

/// Open a folder (or file) with the system's default app: Explorer, Files, Dolphin, ...
#[tauri::command]
pub fn open_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

/// Show a file selected in its folder.
#[tauri::command]
pub fn reveal_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|e| e.to_string())
}
