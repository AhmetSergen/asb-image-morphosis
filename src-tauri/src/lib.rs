// SPDX-License-Identifier: GPL-3.0-or-later
// ASB Image Morphosis - easy batch image converter and resizer.
// Copyright (C) 2026 Ahmet Sergen Boncuk
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

mod commands;
mod naming;
mod options;
mod pipeline;
mod resize;
mod scan;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// State shared between commands.
#[derive(Default)]
pub struct AppState {
    /// Set by `cancel_convert`, checked before each file.
    pub cancel: Arc<AtomicBool>,
    /// Only one batch runs at a time.
    pub running: Arc<AtomicBool>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::scan_paths,
            commands::thumbnail,
            commands::preview_output,
            commands::convert,
            commands::cancel_convert,
            commands::open_path,
            commands::reveal_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ASB Image Morphosis");
}
