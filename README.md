<p align="center">
  <img src="app-icon.svg" width="96" height="96" alt="ASB Image Morphosis icon">
</p>

<h1 align="center">ASB Image Morphosis</h1>

<p align="center">
  An easy batch image converter and resizer for <b>Windows</b> and <b>Linux</b>.<br>
  Free and open source (GPL-3.0-or-later).
</p>

---

Convert and resize many images at once in four steps:

1. **Add images.** Drop files or folders on the window, or use *Add files* / *Add folder*.
2. **Choose a format.** JPEG, PNG, WebP, AVIF, GIF, BMP, TIFF, ICO, or keep each file's own format.
3. **Choose a size.** Keep the original size, fit inside a box, set a width or height, scale by %, crop to fill, or stretch.
4. **Choose where to save** and press **Convert**.

## Features

- **Folders, sub-folders and drag-and-drop.** The app can recreate the sub-folder structure in the output folder.
- **Live preview** of the output name and size before you start.
- **Presets** for common jobs (web, email, thumbnails, social squares, icons, lossless PNG). You can save your own.
- **File name templates** with `{name}`, `{width}`, `{height}`, `{format}`, `{date}`, `{index}`.
- **Safe by default:**
  - Originals are never overwritten.
  - Existing files are kept (`photo (1).webp`) unless you pick *Overwrite* or *Skip*.
  - Files are written atomically, so a cancelled run leaves no half-written images.
- **Phone photos come out upright.** EXIF orientation is applied automatically.
- **Privacy.** Converted images don't carry over metadata such as EXIF or GPS location.
- **Transparency** is kept where the format supports it. For JPEG you choose the background colour.
- **Fast.** Images are processed in parallel on all CPU cores but one, so the app stays responsive.
- **Progress tracking:**
  - Per-file status, and output size with % saved.
  - Cancel at any time.
  - Progress shown on the taskbar.
  - *Open output folder* when done.
- **Remembers settings.** Your settings and last output folder are kept between runs.
- **Light and dark themes** that follow your system.
- **Keyboard shortcuts:**

  | Shortcut | Action |
  |---|---|
  | <kbd>Ctrl</kbd>+<kbd>O</kbd> | Add files |
  | <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>O</kbd> | Add folder |
  | <kbd>Ctrl</kbd>+<kbd>Enter</kbd> | Convert |
  | <kbd>Esc</kbd> | Cancel |

### Supported formats

| | Formats |
|---|---|
| **Reads** | JPEG, PNG, WebP, GIF, BMP, TIFF, ICO, TGA, QOI, PNM (PPM/PGM/PBM/PAM), HDR, OpenEXR |
| **Writes** | JPEG, PNG, WebP (lossy or lossless), AVIF, GIF, BMP, TIFF, ICO (max 256×256) |

**Not supported yet:**
- AVIF and HEIC/HEIF *input*. These need extra native libraries.
- Animated GIFs. Only the first frame is converted.
- Camera RAW, PSD and SVG.

## Built with

| Part | Technologies |
|---|---|
| **App framework** | [Tauri 2](https://tauri.app): a small native app that uses the system web view (WebView2 on Windows, WebKitGTK on Linux) |
| **User interface** | [Svelte 5](https://svelte.dev) (runes), [TypeScript](https://www.typescriptlang.org), [Vite](https://vite.dev) |
| **Image processing** | [Rust](https://www.rust-lang.org) with the [`image`](https://crates.io/crates/image) crate for decoding, resizing and most encoders |
| **WebP and AVIF** | [`webp`](https://crates.io/crates/webp) (libwebp) for lossy WebP; [`ravif`](https://crates.io/crates/ravif) / rav1e for AVIF, via `image` |
| **Parallel work** | [`rayon`](https://crates.io/crates/rayon) to convert several images at once, on all CPU cores but one |
| **Files and folders** | [`walkdir`](https://crates.io/crates/walkdir) for folder scanning; Tauri's dialog and opener plugins for file pickers and *Show in folder* |
| **Builds and releases** | GitHub Actions: checks and tests on pull requests and pushes to `main`, and [tauri-action](https://github.com/tauri-apps/tauri-action) to build the Windows and Linux installers |

## Download

Installers are on the [Releases](https://github.com/AhmetSergen/asb-image-morphosis/releases) page.

| System | File |
|---|---|
| Windows 10/11 | `.msi` or `-setup.exe` |
| Linux (any distro) | `.AppImage`: `chmod +x` it and run |
| Debian / Ubuntu / Mint | `.deb` |
| Fedora / openSUSE | `.rpm` |

## Building from source

The app is built with [Tauri 2](https://tauri.app): a Rust backend does the image work, and a [Svelte 5](https://svelte.dev) + TypeScript frontend provides the UI.

### Prerequisites

You need all of the following:
- [Node.js](https://nodejs.org) 20 or newer
- [Rust](https://rustup.rs) (stable)
- The platform tools for your system:

**Windows:** install the Microsoft C++ Build Tools ("Desktop development with C++"). WebView2 already ships with Windows 10/11.

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install --id Rustlang.Rustup -e
```

**Linux (Debian/Ubuntu):**

```sh
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libssl-dev libxdo-dev build-essential curl wget file patchelf
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

For other distributions, see [Tauri's prerequisites](https://v2.tauri.app/start/prerequisites/).

### Run and build

```sh
npm install
npm run tauri dev      # run in development mode
npm run tauri build    # build installers into src-tauri/target/release/bundle/
```

### Checks

```sh
npm run check                                  # TypeScript / Svelte
cd src-tauri && cargo test                     # Rust unit and conversion tests
cd src-tauri && cargo clippy --all-targets -- -D warnings
```

### Releasing

Push a tag such as `v0.1.0`. The **Release** workflow then builds the Windows and Linux installers on GitHub Actions and attaches them to a draft release, ready to publish. You can also start it by hand from the *Actions* tab.

### Project layout

```
src/                     Svelte UI
  lib/state.svelte.ts    app state (runes), settings persistence
  lib/api.ts             typed wrappers for the Rust commands
  lib/components/        file list, format/size/output panels, action bar
src-tauri/src/
  commands.rs            commands called from the UI
  scan.rs                expand files/folders into images
  pipeline.rs            decode → orient → resize → encode → write
  resize.rs              output size calculations
  naming.rs              file-name templates, output folders, conflict handling
```

## License

Copyright (C) 2026 Ahmet Sergen Boncuk

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. See [LICENSE](LICENSE).
