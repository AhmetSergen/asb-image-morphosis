// SPDX-License-Identifier: GPL-3.0-or-later
// App state (Svelte 5 runes). Settings are remembered in localStorage.

import { getCurrentWindow, ProgressBarStatus } from "@tauri-apps/api/window";
import * as api from "./api";
import { formatBytes } from "./format";
import { BUILT_IN_PRESETS } from "./presets";
import type {
  ConvertOptions,
  FileItem,
  OutputPreview,
  Preset,
  ProgressEvent,
  ResizeMode,
  ResizeModeName,
  Summary,
} from "./types";

const OPTIONS_KEY = "morphosis.options.v1";
const UI_KEY = "morphosis.ui.v1";
const PRESETS_KEY = "morphosis.presets.v1";

export const DEFAULT_OPTIONS: ConvertOptions = {
  format: "webp",
  quality: 82,
  webpLossless: false,
  pngCompression: "default",
  background: "#ffffff",
  resize: { mode: "none" },
  noUpscale: true,
  output: {
    directory: null,
    keepStructure: true,
    nameTemplate: "{name}",
    conflict: "rename",
  },
};

export interface UiPrefs {
  /** "folder" = one chosen folder, "same" = next to each original */
  saveMode: "folder" | "same";
  folder: string | null;
  recursive: boolean;
  /** Remembered box sizes so switching resize modes keeps what you typed. */
  width: number;
  height: number;
  percent: number;
}

const DEFAULT_UI: UiPrefs = {
  saveMode: "folder",
  folder: null,
  recursive: true,
  width: 1920,
  height: 1080,
  percent: 50,
};

function load<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    if (!raw) return structuredClone(fallback);
    const parsed = JSON.parse(raw);
    // Merge so new settings added in later versions get their defaults.
    if (Array.isArray(fallback)) return parsed as T;
    return {
      ...structuredClone(fallback),
      ...parsed,
      ...("output" in (fallback as object)
        ? { output: { ...(fallback as ConvertOptions).output, ...parsed.output } }
        : {}),
    };
  } catch {
    return structuredClone(fallback);
  }
}

export interface Toast {
  id: number;
  kind: "info" | "error" | "success";
  text: string;
  detail?: string[];
}

class AppState {
  files = $state<FileItem[]>([]);

  options = $state<ConvertOptions>(load(OPTIONS_KEY, DEFAULT_OPTIONS));
  ui = $state<UiPrefs>(load(UI_KEY, DEFAULT_UI));
  userPresets = $state<Preset[]>(load<Preset[]>(PRESETS_KEY, []));

  scanning = $state(false);
  running = $state(false);
  cancelling = $state(false);
  progress = $state({ done: 0, total: 0 });
  summary = $state<Summary | null>(null);
  preview = $state<OutputPreview | null>(null);
  previewError = $state<string | null>(null);
  toasts = $state<Toast[]>([]);

  #toastId = 0;
  #index = new Map<string, FileItem>();

  totalBytes = $derived(this.files.reduce((sum, f) => sum + f.sizeBytes, 0));
  presets = $derived([...BUILT_IN_PRESETS, ...this.userPresets]);

  /** Options as sent to Rust (output folder depends on the "save to" choice). */
  effectiveOptions = $derived<ConvertOptions>({
    ...this.options,
    output: {
      ...this.options.output,
      directory: this.ui.saveMode === "same" ? null : this.ui.folder,
    },
  });

  /** Why the Convert button is disabled, or null if it's ready. */
  blocker = $derived.by((): string | null => {
    if (this.files.length === 0) return "Add some images first";
    if (this.ui.saveMode === "folder" && !this.ui.folder) return "Choose an output folder";
    const r = this.options.resize;
    const bad = (n: number) => !Number.isFinite(n) || n <= 0;
    if (r.mode === "percent" && bad(r.percent)) return "Enter a scale percentage";
    if (r.mode === "width" && bad(r.width)) return "Enter a width";
    if (r.mode === "height" && bad(r.height)) return "Enter a height";
    if (r.mode === "fit" && bad(r.width) && bad(r.height)) return "Enter a width or height";
    if ((r.mode === "fill" || r.mode === "stretch") && (bad(r.width) || bad(r.height)))
      return "Enter both width and height";
    return null;
  });

  constructor() {
    $effect.root(() => {
      $effect(() => {
        localStorage.setItem(OPTIONS_KEY, JSON.stringify(this.options));
      });
      $effect(() => {
        localStorage.setItem(UI_KEY, JSON.stringify(this.ui));
      });
      $effect(() => {
        localStorage.setItem(PRESETS_KEY, JSON.stringify(this.userPresets));
      });
    });
  }

  // ---------- toasts ----------

  toast(kind: Toast["kind"], text: string, detail?: string[], ms = 5000) {
    const id = ++this.#toastId;
    this.toasts.push({ id, kind, text, detail });
    if (ms > 0) setTimeout(() => this.dismiss(id), ms);
  }

  dismiss(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  // ---------- file list ----------

  #reindex() {
    this.#index = new Map(this.files.map((f) => [f.path, f]));
  }

  async addPaths(paths: string[]) {
    if (paths.length === 0 || this.running) return;
    this.scanning = true;
    try {
      const result = await api.scanPaths(paths, this.ui.recursive);
      const fresh = result.entries.filter((e) => !this.#index.has(e.path));
      const duplicates = result.entries.length - fresh.length;
      // concat (not push(...spread)) so huge folders can't overflow the call stack.
      this.files = this.files.concat(
        fresh.map((e) => ({ ...e, status: { state: "queued" as const } })),
      );
      this.#reindex();
      this.summary = null;

      const notes: string[] = [];
      if (duplicates > 0) notes.push(`${duplicates} already in the list`);
      if (result.ignored > 0) notes.push(`${result.ignored} non-image files ignored`);
      if (fresh.length > 0) {
        this.toast("success", `Added ${fresh.length} image${fresh.length === 1 ? "" : "s"}`, notes, 3000);
      } else if (result.skipped.length === 0) {
        this.toast("info", "No new images found", notes);
      }
      if (result.skipped.length > 0) {
        this.toast(
          "error",
          `${result.skipped.length} file${result.skipped.length === 1 ? "" : "s"} couldn't be added`,
          result.skipped.slice(0, 6).map((s) => `${basename(s.path)}: ${s.reason}`),
          9000,
        );
      }
    } catch (e) {
      this.toast("error", "Couldn't read the selected items", [String(e)]);
    } finally {
      this.scanning = false;
    }
  }

  async addFiles() {
    this.addPaths(await api.pickFiles());
  }

  async addFolders() {
    this.addPaths(await api.pickFolders());
  }

  remove(paths: Iterable<string>) {
    if (this.running) return;
    const gone = new Set(paths);
    this.files = this.files.filter((f) => !gone.has(f.path));
    this.#reindex();
  }

  removeConverted() {
    this.remove(this.files.filter((f) => f.status.state === "done").map((f) => f.path));
    this.summary = null;
  }

  clear() {
    if (this.running) return;
    this.files = [];
    this.summary = null;
    this.#reindex();
  }

  // ---------- settings ----------

  setResizeMode(mode: ResizeModeName) {
    const { width, height, percent } = this.ui;
    const next: Record<ResizeModeName, ResizeMode> = {
      none: { mode: "none" },
      percent: { mode: "percent", percent },
      fit: { mode: "fit", width, height },
      width: { mode: "width", width },
      height: { mode: "height", height },
      fill: { mode: "fill", width, height },
      stretch: { mode: "stretch", width, height },
    };
    this.options.resize = next[mode];
  }

  /** Update one number of the current resize mode (if that mode has it). */
  setDim(key: "width" | "height" | "percent", value: number) {
    const r = this.options.resize as Record<string, unknown>;
    if (key in r) {
      r[key] = Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0;
      this.rememberSize();
    }
  }

  /** Keep typed sizes so they survive switching modes. */
  rememberSize() {
    const r = this.options.resize;
    if ("width" in r) this.ui.width = r.width;
    if ("height" in r) this.ui.height = r.height;
    if ("percent" in r) this.ui.percent = r.percent;
  }

  applyPreset(preset: Preset) {
    Object.assign(this.options, structuredClone($state.snapshot(preset.settings)));
    this.rememberSize();
  }

  savePreset(name: string) {
    const { format, quality, webpLossless, pngCompression, background, resize, noUpscale } =
      $state.snapshot(this.options);
    this.userPresets.push({
      id: `user-${Date.now()}`,
      name,
      settings: { format, quality, webpLossless, pngCompression, background, resize, noUpscale },
    });
  }

  deletePreset(id: string) {
    this.userPresets = this.userPresets.filter((p) => p.id !== id);
  }

  resetSettings() {
    this.options = structuredClone(DEFAULT_OPTIONS);
  }

  async chooseFolder() {
    const folder = await api.pickOutputFolder(this.ui.folder);
    if (folder) {
      this.ui.folder = folder;
      this.ui.saveMode = "folder";
    }
  }

  // ---------- conversion ----------

  async convert() {
    if (this.running || this.blocker) return;
    this.running = true;
    this.cancelling = false;
    this.summary = null;
    this.progress = { done: 0, total: this.files.length };
    for (const f of this.files) f.status = { state: "queued" };
    this.#taskbar(0);

    const items = this.files.map((f) => ({ path: f.path, relDir: f.relDir }));
    try {
      this.summary = await api.convert(items, $state.snapshot(this.effectiveOptions), (e) =>
        this.#onEvent(e),
      );
      const s = this.summary;
      if (s.failed > 0) {
        this.toast("error", `${s.failed} image${s.failed === 1 ? "" : "s"} failed`, [
          "Hover the red status chips for details.",
        ]);
      }
    } catch (e) {
      this.toast("error", "Conversion failed", [String(e)]);
    } finally {
      this.running = false;
      this.cancelling = false;
      this.#taskbar(null);
    }
  }

  #onEvent(e: ProgressEvent) {
    switch (e.event) {
      case "fileStarted": {
        const f = this.#index.get(e.data.path);
        if (f) f.status = { state: "working" };
        return;
      }
      case "fileDone": {
        const f = this.#index.get(e.data.path);
        const { outputPath, outBytes, width, height } = e.data;
        if (f) f.status = { state: "done", outputPath, outBytes, width, height };
        break;
      }
      case "fileSkipped": {
        const f = this.#index.get(e.data.path);
        if (f) f.status = { state: "skipped", reason: e.data.reason };
        break;
      }
      case "fileFailed": {
        const f = this.#index.get(e.data.path);
        if (f) f.status = { state: "failed", error: e.data.error };
        break;
      }
      default:
        return;
    }
    this.progress.done++;
    this.#taskbar(this.progress.done / Math.max(1, this.progress.total));
  }

  async cancel() {
    if (!this.running) return;
    this.cancelling = true;
    await api.cancelConvert();
  }

  /** Mirror progress on the taskbar / dock (Windows, some Linux desktops). */
  #taskbar(fraction: number | null) {
    const win = getCurrentWindow();
    const p =
      fraction === null
        ? win.setProgressBar({ status: ProgressBarStatus.None })
        : win.setProgressBar({ status: ProgressBarStatus.Normal, progress: Math.round(fraction * 100) });
    p.catch(() => {});
  }

  async refreshPreview() {
    const first = this.files[0];
    // Still runs without an output folder: the size part is used by the resize panel.
    if (!first) {
      this.preview = null;
      this.previewError = null;
      return;
    }
    try {
      this.preview = await api.previewOutput(
        { path: first.path, relDir: first.relDir },
        $state.snapshot(this.effectiveOptions),
      );
      this.previewError = null;
    } catch (e) {
      this.preview = null;
      this.previewError = String(e);
    }
  }

  summaryText(s: Summary): string {
    const parts = [`${s.converted} converted`];
    if (s.skipped) parts.push(`${s.skipped} skipped`);
    if (s.failed) parts.push(`${s.failed} failed`);
    if (s.converted > 0) {
      const diff = s.inBytes - s.outBytes;
      parts.push(diff >= 0 ? `saved ${formatBytes(diff)}` : `${formatBytes(-diff)} larger`);
    }
    return parts.join(" · ");
  }
}

export function basename(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export const app = new AppState();
