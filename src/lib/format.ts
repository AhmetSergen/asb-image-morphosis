// SPDX-License-Identifier: GPL-3.0-or-later
// Small display helpers and option metadata.

import type { ConflictPolicy, OutputFormat, ResizeModeName } from "./types";

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const v = bytes / 1024 ** i;
  return `${v >= 100 || i === 0 ? v.toFixed(0) : v.toFixed(1)} ${units[i]}`;
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms} ms`;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(1)} s`;
  return `${Math.floor(s / 60)} min ${Math.round(s % 60)} s`;
}

/** `-42%` (smaller) or `+8%` (larger). */
export function formatChange(before: number, after: number): string {
  if (before <= 0) return "";
  // trunc, so a 99.9 % saving reads -99 % rather than a misleading -100 %.
  const pct = Math.trunc(((after - before) / before) * 100);
  return `${pct > 0 ? "+" : ""}${pct}%`;
}

export const FORMATS: { value: OutputFormat; label: string; hint: string }[] = [
  { value: "original", label: "Keep", hint: "Keep each file's own format" },
  { value: "jpeg", label: "JPEG", hint: "Photos; small files, no transparency" },
  { value: "png", label: "PNG", hint: "Lossless, supports transparency" },
  { value: "webp", label: "WebP", hint: "Modern web format; small, supports transparency" },
  { value: "avif", label: "AVIF", hint: "Smallest files; slower to encode" },
  { value: "gif", label: "GIF", hint: "256 colours; first frame only" },
  { value: "bmp", label: "BMP", hint: "Uncompressed bitmap" },
  { value: "tiff", label: "TIFF", hint: "Lossless, for printing and archiving" },
  { value: "ico", label: "ICO", hint: "Windows icon, max 256×256" },
];

export const RESIZE_MODES: { value: ResizeModeName; label: string }[] = [
  { value: "none", label: "Original size" },
  { value: "fit", label: "Fit inside box" },
  { value: "width", label: "Set width" },
  { value: "height", label: "Set height" },
  { value: "percent", label: "Scale by %" },
  { value: "fill", label: "Fill box (crop)" },
  { value: "stretch", label: "Stretch to exact size" },
];

export const CONFLICTS: { value: ConflictPolicy; label: string }[] = [
  { value: "rename", label: "Keep both (add number)" },
  { value: "overwrite", label: "Overwrite" },
  { value: "skip", label: "Skip file" },
];

export const NAME_TOKENS = ["{name}", "{width}", "{height}", "{format}", "{date}", "{index}"];

/** Formats where the quality slider applies. */
export function usesQuality(format: OutputFormat, webpLossless: boolean): boolean {
  return format === "jpeg" || format === "avif" || (format === "webp" && !webpLossless);
}
