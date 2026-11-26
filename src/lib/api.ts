// SPDX-License-Identifier: GPL-3.0-or-later
// Typed wrappers around the Rust commands in src-tauri/src/commands.rs.

import { Channel, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  ConvertItem,
  ConvertOptions,
  OutputPreview,
  ProgressEvent,
  ScanResult,
  Summary,
} from "./types";

/** Extensions the backend can read (keep in sync with scan.rs). */
export const READABLE_EXTENSIONS = [
  "jpg", "jpeg", "jpe", "jfif", "png", "gif", "bmp", "dib", "tif", "tiff", "webp", "ico",
  "tga", "qoi", "ppm", "pgm", "pbm", "pnm", "pam", "hdr", "exr",
];

export const scanPaths = (paths: string[], recursive: boolean) =>
  invoke<ScanResult>("scan_paths", { paths, recursive });

export const thumbnail = (path: string, size: number) =>
  invoke<string>("thumbnail", { path, size });

export const previewOutput = (item: ConvertItem, options: ConvertOptions) =>
  invoke<OutputPreview>("preview_output", { item, options });

export function convert(
  files: ConvertItem[],
  options: ConvertOptions,
  onEvent: (e: ProgressEvent) => void,
): Promise<Summary> {
  const onProgress = new Channel<ProgressEvent>();
  onProgress.onmessage = onEvent;
  return invoke<Summary>("convert", { files, options, onProgress });
}

export const cancelConvert = () => invoke<void>("cancel_convert");
export const openPath = (path: string) => invoke<void>("open_path", { path });
export const revealPath = (path: string) => invoke<void>("reveal_path", { path });

export async function pickFiles(): Promise<string[]> {
  const picked = await open({
    multiple: true,
    directory: false,
    title: "Add images",
    filters: [
      { name: "Images", extensions: READABLE_EXTENSIONS },
      { name: "All files", extensions: ["*"] },
    ],
  });
  return picked ?? [];
}

export async function pickFolders(): Promise<string[]> {
  const picked = await open({ multiple: true, directory: true, title: "Add folders" });
  return picked ?? [];
}

export async function pickOutputFolder(current?: string | null): Promise<string | null> {
  const picked = await open({
    multiple: false,
    directory: true,
    title: "Choose where to save converted images",
    defaultPath: current ?? undefined,
  });
  return picked ?? null;
}
