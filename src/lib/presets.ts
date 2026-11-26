// SPDX-License-Identifier: GPL-3.0-or-later
import type { Preset } from "./types";

export const BUILT_IN_PRESETS: Preset[] = [
  {
    id: "web",
    name: "Web optimised (WebP, max 1920 px)",
    builtIn: true,
    settings: {
      format: "webp",
      quality: 80,
      webpLossless: false,
      pngCompression: "default",
      background: "#ffffff",
      resize: { mode: "fit", width: 1920, height: 1920 },
      noUpscale: true,
    },
  },
  {
    id: "email",
    name: "Email / sharing (JPEG, max 1600 px)",
    builtIn: true,
    settings: {
      format: "jpeg",
      quality: 82,
      webpLossless: false,
      pngCompression: "default",
      background: "#ffffff",
      resize: { mode: "fit", width: 1600, height: 1600 },
      noUpscale: true,
    },
  },
  {
    id: "thumb",
    name: "Thumbnail (JPEG, max 320 px)",
    builtIn: true,
    settings: {
      format: "jpeg",
      quality: 85,
      webpLossless: false,
      pngCompression: "default",
      background: "#ffffff",
      resize: { mode: "fit", width: 320, height: 320 },
      noUpscale: true,
    },
  },
  {
    id: "square",
    name: "Social square (JPEG, 1080×1080 crop)",
    builtIn: true,
    settings: {
      format: "jpeg",
      quality: 90,
      webpLossless: false,
      pngCompression: "default",
      background: "#ffffff",
      resize: { mode: "fill", width: 1080, height: 1080 },
      noUpscale: false,
    },
  },
  {
    id: "favicon",
    name: "Icon (ICO, 256 px)",
    builtIn: true,
    settings: {
      format: "ico",
      quality: 90,
      webpLossless: false,
      pngCompression: "default",
      background: "#ffffff",
      resize: { mode: "fill", width: 256, height: 256 },
      noUpscale: false,
    },
  },
  {
    id: "png",
    name: "Lossless PNG (original size)",
    builtIn: true,
    settings: {
      format: "png",
      quality: 90,
      webpLossless: false,
      pngCompression: "best",
      background: "#ffffff",
      resize: { mode: "none" },
      noUpscale: true,
    },
  },
];
