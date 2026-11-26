// SPDX-License-Identifier: GPL-3.0-or-later
// Mirrors the serde types in src-tauri/src/options.rs.

export type OutputFormat =
  | "original"
  | "jpeg"
  | "png"
  | "webp"
  | "avif"
  | "gif"
  | "bmp"
  | "tiff"
  | "ico";

export type ResizeMode =
  | { mode: "none" }
  | { mode: "percent"; percent: number }
  | { mode: "fit"; width: number; height: number }
  | { mode: "width"; width: number }
  | { mode: "height"; height: number }
  | { mode: "fill"; width: number; height: number }
  | { mode: "stretch"; width: number; height: number };

export type ResizeModeName = ResizeMode["mode"];
export type PngCompression = "fast" | "default" | "best";
export type ConflictPolicy = "rename" | "overwrite" | "skip";

export interface OutputOptions {
  /** null = next to each original file */
  directory: string | null;
  keepStructure: boolean;
  nameTemplate: string;
  conflict: ConflictPolicy;
}

export interface ConvertOptions {
  format: OutputFormat;
  quality: number;
  webpLossless: boolean;
  pngCompression: PngCompression;
  background: string;
  resize: ResizeMode;
  noUpscale: boolean;
  output: OutputOptions;
}

export interface ConvertItem {
  path: string;
  relDir: string;
}

export interface ImageEntry {
  path: string;
  name: string;
  ext: string;
  relDir: string;
  sizeBytes: number;
  width: number;
  height: number;
}

export interface SkippedFile {
  path: string;
  reason: string;
}

export interface ScanResult {
  entries: ImageEntry[];
  skipped: SkippedFile[];
  ignored: number;
}

export interface Summary {
  converted: number;
  skipped: number;
  failed: number;
  cancelled: boolean;
  inBytes: number;
  outBytes: number;
  elapsedMs: number;
  outputDirs: string[];
}

export type ProgressEvent =
  | { event: "started"; data: { total: number } }
  | { event: "fileStarted"; data: { path: string } }
  | {
      event: "fileDone";
      data: { path: string; outputPath: string; outBytes: number; width: number; height: number };
    }
  | { event: "fileSkipped"; data: { path: string; reason: string } }
  | { event: "fileFailed"; data: { path: string; error: string } }
  | { event: "finished"; data: { summary: Summary } };

export interface OutputPreview {
  path: string;
  width: number;
  height: number;
  exists: boolean;
  isSource: boolean;
}

/** Per-file state shown in the list. */
export type FileStatus =
  | { state: "queued" }
  | { state: "working" }
  | { state: "done"; outputPath: string; outBytes: number; width: number; height: number }
  | { state: "skipped"; reason: string }
  | { state: "failed"; error: string };

export interface FileItem extends ImageEntry {
  status: FileStatus;
}

export interface Preset {
  id: string;
  name: string;
  builtIn?: boolean;
  /** Only the conversion settings; the output folder is never part of a preset. */
  settings: Pick<
    ConvertOptions,
    "format" | "quality" | "webpLossless" | "pngCompression" | "background" | "resize" | "noUpscale"
  >;
}
