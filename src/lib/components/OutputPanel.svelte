<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { openPath } from "../api";
  import { CONFLICTS, NAME_TOKENS } from "../format";
  import { app, basename } from "../state.svelte";
  import Icon from "./Icon.svelte";

  let templateInput: HTMLInputElement | undefined = $state();

  function insertToken(token: string) {
    const el = templateInput;
    const t = app.options.output.nameTemplate;
    const start = el?.selectionStart ?? t.length;
    const end = el?.selectionEnd ?? t.length;
    app.options.output.nameTemplate = t.slice(0, start) + token + t.slice(end);
    requestAnimationFrame(() => {
      el?.focus();
      el?.setSelectionRange(start + token.length, start + token.length);
    });
  }

  // Live preview of the first file's output, debounced while typing.
  $effect(() => {
    // Track everything the preview depends on.
    JSON.stringify(app.effectiveOptions);
    void app.files[0]?.path;
    const t = setTimeout(() => app.refreshPreview(), 200);
    return () => clearTimeout(t);
  });

  let conflictNote = $derived.by(() => {
    const p = app.preview;
    if (!p?.exists) return null;
    if (p.isSource) return "That's the original file. Originals are never overwritten, so a number will be added.";
    switch (app.options.output.conflict) {
      case "rename":
        return "A file with this name exists. A number will be added.";
      case "overwrite":
        return "A file with this name exists and will be replaced.";
      case "skip":
        return "A file with this name exists, so this image will be skipped.";
    }
  });
</script>

<fieldset class="field">
  <legend class="label">Save to</legend>
  <label class="check">
    <input type="radio" name="save" value="folder" bind:group={app.ui.saveMode} />
    A folder of my choice
  </label>
  {#if app.ui.saveMode === "folder"}
    <div class="folder">
      <button class="path" class:empty={!app.ui.folder} onclick={() => app.chooseFolder()} title={app.ui.folder ?? ""}>
        <Icon name="folder" />
        <span class="path-text"><bdi>{app.ui.folder ?? "Choose a folder…"}</bdi></span>
      </button>
      <button class="btn small" onclick={() => app.chooseFolder()}>Browse…</button>
      {#if app.ui.folder}
        <button
          class="icon-btn"
          title="Open this folder"
          onclick={() => app.ui.folder && openPath(app.ui.folder).catch((e) => app.toast("error", String(e)))}
        >
          <Icon name="external" />
        </button>
      {/if}
    </div>
    <label class="check indent">
      <input type="checkbox" bind:checked={app.options.output.keepStructure} />
      Keep sub-folder structure
    </label>
  {/if}
  <label class="check">
    <input type="radio" name="save" value="same" bind:group={app.ui.saveMode} />
    Next to each original
  </label>
</fieldset>

<div class="field">
  <label class="label" for="template">File name</label>
  <input
    id="template"
    type="text"
    spellcheck="false"
    bind:this={templateInput}
    bind:value={app.options.output.nameTemplate}
    placeholder={"{name}"}
  />
  <div class="tokens">
    {#each NAME_TOKENS as token (token)}
      <button class="token" onclick={() => insertToken(token)} title="Insert {token}">{token}</button>
    {/each}
  </div>
</div>

<div class="field">
  <label class="label" for="conflict">If a file already exists</label>
  <select id="conflict" bind:value={app.options.output.conflict}>
    {#each CONFLICTS as c (c.value)}
      <option value={c.value}>{c.label}</option>
    {/each}
  </select>
</div>

{#if app.preview && app.files[0] && !(app.ui.saveMode === "folder" && !app.ui.folder)}
  <div class="preview" aria-live="polite">
    <div class="label">Example</div>
    <div class="mono line" title={app.files[0].path}>{app.files[0].name}</div>
    <div class="mono line out" title={app.preview.path}>
      <Icon name="arrowRight" size={13} />
      {basename(app.preview.path)}
      <span class="dims">{app.preview.width}×{app.preview.height}</span>
    </div>
    {#if conflictNote}<div class="note">{conflictNote}</div>{/if}
  </div>
{:else if app.previewError}
  <div class="preview"><div class="note">{app.previewError}</div></div>
{/if}

<style>
  fieldset {
    gap: 8px;
    margin: 0;
    padding: 0;
    border: none;
  }

  legend {
    margin-bottom: 6px;
    padding: 0;
  }

  .folder {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: 24px;
  }

  .path {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 6px;
    min-width: 0;
    min-height: 30px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }

  .path.empty {
    color: var(--accent);
    font-weight: 600;
  }

  .path-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl; /* show the end of long paths */
  }

  .indent {
    margin-left: 24px;
    font-size: 13px;
  }

  .tokens {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .preview {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .line {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .out {
    color: var(--accent);
  }

  .dims {
    margin-left: auto;
    padding-left: 8px;
    color: var(--muted);
  }

  .note {
    margin-top: 4px;
    font-size: 12px;
    color: var(--warn);
  }
</style>
