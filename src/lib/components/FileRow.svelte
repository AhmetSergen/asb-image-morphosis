<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { revealPath } from "../api";
  import { formatBytes, formatChange } from "../format";
  import { app } from "../state.svelte";
  import { cachedThumb, loadThumb, whenVisible } from "../thumbs";
  import type { FileItem } from "../types";
  import Icon from "./Icon.svelte";

  let { file, selected }: { file: FileItem; selected: boolean } = $props();

  let thumb = $state<string | null | undefined>(undefined);

  function showThumb() {
    const hit = cachedThumb(file.path);
    if (hit) thumb = hit;
    else loadThumb(file.path).then((url) => (thumb = url));
  }

  function reveal(e: MouseEvent) {
    e.stopPropagation();
    if (file.status.state === "done") revealPath(file.status.outputPath).catch(() => {});
  }
</script>

<div
  class="row"
  class:selected
  role="option"
  tabindex="-1"
  aria-selected={selected}
  onclick={(e) => app.select(file.path, e)}
  ondblclick={reveal}
  onkeydown={(e) => e.key === " " && app.select(file.path, e)}
>
  <div class="thumb" use:whenVisible={showThumb}>
    {#if thumb}
      <img src={thumb} alt="" draggable="false" />
    {:else}
      <Icon name="image" size={20} />
    {/if}
  </div>

  <div class="info">
    <div class="name" title={file.path}>{file.name}</div>
    <div class="meta">
      <span class="badge">{file.ext.toUpperCase()}</span>
      {file.width}×{file.height} · {formatBytes(file.sizeBytes)}
      {#if file.relDir}<span class="dir" title={file.relDir}>· {file.relDir}</span>{/if}
    </div>
  </div>

  <div class="status">
    {#if file.status.state === "done"}
      <span class="chip ok" title={`Saved to ${file.status.outputPath}`}>
        <Icon name="check" size={13} />
        {file.status.width}×{file.status.height} · {formatBytes(file.status.outBytes)}
        <span class="change">{formatChange(file.sizeBytes, file.status.outBytes)}</span>
      </span>
      <button class="icon-btn" title="Show in folder" onclick={reveal}>
        <Icon name="external" size={15} />
      </button>
    {:else if file.status.state === "skipped"}
      <span class="chip warn" title={file.status.reason}><Icon name="skip" size={13} /> Skipped</span>
    {:else if file.status.state === "failed"}
      <span class="chip err" title={file.status.error}><Icon name="alert" size={13} /> Failed</span>
    {:else if file.status.state === "working"}
      <span class="chip busy"><span class="spinner" aria-hidden="true"></span> Converting…</span>
    {:else if app.running}
      <span class="chip wait">Waiting…</span>
    {/if}
  </div>

  <button
    class="icon-btn remove"
    title="Remove from list"
    aria-label="Remove {file.name} from list"
    disabled={app.running}
    onclick={(e) => {
      e.stopPropagation();
      app.remove([file.path]);
    }}
  >
    <Icon name="x" size={15} />
  </button>
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 44px minmax(0, 1fr) auto 28px;
    align-items: center;
    gap: 12px;
    padding: 6px 10px;
    border-radius: 8px;
    cursor: default;
    content-visibility: auto;
    contain-intrinsic-size: auto 56px;
  }

  .row:hover {
    background: var(--hover);
  }

  .row.selected {
    background: var(--accent-soft);
  }

  .thumb {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    overflow: hidden;
    border-radius: 6px;
    background: var(--surface-2);
    color: var(--muted);
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .info {
    min-width: 0;
  }

  .name {
    overflow: hidden;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    overflow: hidden;
    font-size: 12px;
    color: var(--muted);
    text-overflow: ellipsis;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .badge {
    display: inline-block;
    margin-right: 4px;
    padding: 0 5px;
    border-radius: 4px;
    background: var(--surface-2);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.03em;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .chip.ok {
    background: var(--success-soft);
    color: var(--success);
  }

  .chip.warn {
    background: var(--warn-soft);
    color: var(--warn);
  }

  .chip.err {
    background: var(--danger-soft);
    color: var(--danger);
  }

  .chip.wait {
    color: var(--muted);
  }

  .chip.busy {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .spinner {
    width: 11px;
    height: 11px;
    border: 2px solid currentColor;
    border-right-color: transparent;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .change {
    opacity: 0.8;
  }

  .remove {
    opacity: 0;
  }

  .row:hover .remove,
  .row.selected .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  .remove:disabled {
    visibility: hidden;
  }
</style>
