// SPDX-License-Identifier: GPL-3.0-or-later
// Lazy thumbnail loading: only rows on screen ask for one, a few at a time.

import { thumbnail } from "./api";

const SIZE = 96;
const MAX_PARALLEL = 3;

const cache = new Map<string, string>();
const failed = new Set<string>();
const queue: { path: string; resolve: (url: string | null) => void }[] = [];
let active = 0;

function pump() {
  while (active < MAX_PARALLEL && queue.length > 0) {
    const job = queue.shift()!;
    active++;
    thumbnail(job.path, SIZE)
      .then((url) => {
        cache.set(job.path, url);
        job.resolve(url);
      })
      .catch(() => {
        failed.add(job.path);
        job.resolve(null);
      })
      .finally(() => {
        active--;
        pump();
      });
  }
}

export function cachedThumb(path: string): string | undefined {
  return cache.get(path);
}

export function loadThumb(path: string): Promise<string | null> {
  const hit = cache.get(path);
  if (hit) return Promise.resolve(hit);
  if (failed.has(path)) return Promise.resolve(null);
  return new Promise((resolve) => {
    queue.push({ path, resolve });
    pump();
  });
}

/** Svelte action: call `onVisible` once when the element scrolls into view. */
export function whenVisible(node: HTMLElement, onVisible: () => void) {
  const io = new IntersectionObserver(
    (entries) => {
      if (entries.some((e) => e.isIntersecting)) {
        io.disconnect();
        onVisible();
      }
    },
    { rootMargin: "200px" },
  );
  io.observe(node);
  return { destroy: () => io.disconnect() };
}
