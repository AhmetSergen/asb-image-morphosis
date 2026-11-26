<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { formatBytes } from "../format";
  import { app } from "../state.svelte";
  import FileRow from "./FileRow.svelte";
  import Icon from "./Icon.svelte";

  let doneCount = $derived(app.files.filter((f) => f.status.state === "done").length);
</script>

<section class="files" aria-label="Images to convert">
  <div class="toolbar">
    <button class="btn" onclick={() => app.addFiles()} disabled={app.running} title="Add images (Ctrl+O)">
      <Icon name="plus" /> Add files
    </button>
    <button
      class="btn"
      onclick={() => app.addFolders()}
      disabled={app.running}
      title="Add a folder (Ctrl+Shift+O)"
    >
      <Icon name="folder" /> Add folder
    </button>
    <label class="check sub" title="When adding folders, also include images in their sub-folders">
      <input type="checkbox" bind:checked={app.ui.recursive} />
      Include sub-folders
    </label>

    <div class="spacer"></div>

    {#if app.files.length > 0}
      <span class="count">
        {app.files.length} image{app.files.length === 1 ? "" : "s"} · {formatBytes(app.totalBytes)}
      </span>
      {#if doneCount > 0 && !app.running}
        <button class="btn ghost small" onclick={() => app.removeConverted()} title="Remove converted images from the list">
          Remove done
        </button>
      {/if}
      {#if app.selected.size > 0}
        <button class="btn ghost small" onclick={() => app.removeSelected()} disabled={app.running}>
          Remove selected ({app.selected.size})
        </button>
      {/if}
      <button class="btn ghost small danger" onclick={() => app.clear()} disabled={app.running}>
        <Icon name="trash" size={14} /> Clear
      </button>
    {/if}
  </div>

  {#if app.files.length === 0}
    <div class="empty">
      <div class="drop">
        <div class="drop-icon"><Icon name="upload" size={34} /></div>
        <h2>Drop images or folders here</h2>
        <p class="hint">or</p>
        <div class="actions">
          <button class="btn primary" onclick={() => app.addFiles()}>
            <Icon name="file" /> Choose files
          </button>
          <button class="btn" onclick={() => app.addFolders()}>
            <Icon name="folder" /> Choose a folder
          </button>
        </div>
        <p class="hint formats">
          Reads JPEG, PNG, WebP, GIF, BMP, TIFF, ICO, TGA, QOI, PNM, HDR and EXR
        </p>
      </div>
    </div>
  {:else}
    <div class="list" role="listbox" aria-multiselectable="true" aria-label="Images">
      {#each app.files as file (file.path)}
        <FileRow {file} selected={app.selected.has(file.path)} />
      {/each}
    </div>
  {/if}

  {#if app.scanning}
    <div class="scanning" role="status">Reading files…</div>
  {/if}
</section>

<style>
  .files {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow);
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }

  .sub {
    margin-left: 4px;
    font-size: 13px;
    color: var(--muted);
  }

  .spacer {
    flex: 1;
  }

  .sub {
    white-space: nowrap;
  }

  .count {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }

  .list {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
  }

  .empty {
    display: grid;
    flex: 1;
    padding: 16px;
  }

  .drop {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 24px;
    border: 2px dashed var(--border);
    border-radius: var(--radius);
    text-align: center;
  }

  .drop-icon {
    display: grid;
    place-items: center;
    width: 68px;
    height: 68px;
    margin-bottom: 6px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent);
  }

  h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }

  .drop p {
    margin: 0;
  }

  .actions {
    display: flex;
    gap: 8px;
    margin: 4px 0 10px;
  }

  .formats {
    max-width: 340px;
  }

  .scanning {
    position: absolute;
    inset: auto 12px 12px auto;
    padding: 6px 12px;
    border-radius: 999px;
    background: var(--text);
    color: var(--bg);
    font-size: 12px;
  }
</style>
