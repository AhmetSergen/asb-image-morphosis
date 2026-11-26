<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import logo from "../app-icon.svg";
  import ActionBar from "./lib/components/ActionBar.svelte";
  import FileList from "./lib/components/FileList.svelte";
  import FormatPanel from "./lib/components/FormatPanel.svelte";
  import Icon from "./lib/components/Icon.svelte";
  import OutputPanel from "./lib/components/OutputPanel.svelte";
  import PresetPicker from "./lib/components/PresetPicker.svelte";
  import ResizePanel from "./lib/components/ResizePanel.svelte";
  import Toasts from "./lib/components/Toasts.svelte";
  import { app } from "./lib/state.svelte";

  const REPO = "https://github.com/AhmetSergen/asb-image-morphosis";

  let dragging = $state(false);
  let version = $state("");
  let about: HTMLDialogElement | undefined = $state();

  onMount(() => {
    getVersion().then((v) => (version = v)).catch(() => {});

    let unlisten: (() => void) | undefined;
    let disposed = false;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "enter" || p.type === "over") {
          dragging = !app.running;
        } else if (p.type === "drop") {
          dragging = false;
          app.addPaths(p.paths);
        } else {
          dragging = false;
        }
      })
      .then((fn) => (disposed ? fn() : (unlisten = fn)))
      .catch(() => {});

    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  function isTyping(target: EventTarget | null) {
    const el = target as HTMLElement | null;
    return !!el && (el.tagName === "INPUT" || el.tagName === "SELECT" || el.tagName === "TEXTAREA");
  }

  function onKeydown(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    const key = e.key.toLowerCase();
    if (mod && key === "o") {
      e.preventDefault();
      if (e.shiftKey) app.addFolders();
      else app.addFiles();
    } else if (mod && key === "enter") {
      e.preventDefault();
      app.convert();
    } else if (key === "escape" && app.running) {
      app.cancel();
    } else if (isTyping(e.target)) {
      return;
    } else if ((key === "delete" || key === "backspace") && app.selected.size > 0) {
      e.preventDefault();
      app.removeSelected();
    } else if (mod && key === "a" && app.files.length > 0) {
      e.preventDefault();
      app.selectAll();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="app">
  <header>
    <img class="logo" src={logo} alt="" width="26" height="26" />
    <h1>ASB Image Morphosis</h1>
    <div class="spacer"></div>
    <button class="icon-btn" title="About" aria-label="About" onclick={() => about?.showModal()}>
      <Icon name="info" size={18} />
    </button>
  </header>

  <main>
    <FileList />

    <aside class="settings" aria-label="Conversion settings">
      <PresetPicker />

      <section>
        <h2><span class="step">1</span> Format</h2>
        <FormatPanel />
      </section>

      <section>
        <h2><span class="step">2</span> Size</h2>
        <ResizePanel />
      </section>

      <section>
        <h2><span class="step">3</span> Output</h2>
        <OutputPanel />
      </section>
    </aside>
  </main>

  <ActionBar />
</div>

{#if dragging}
  <div class="drag-overlay" aria-hidden="true">
    <div class="drag-card">
      <Icon name="upload" size={40} />
      <div>Drop to add images</div>
      <div class="hint">{app.ui.recursive ? "Folders include their sub-folders" : "Only top-level files of folders"}</div>
    </div>
  </div>
{/if}

<Toasts />

<dialog bind:this={about} onclick={(e) => e.target === about && about?.close()}>
  <div class="about">
    <img src={logo} alt="" width="72" height="72" />
    <h2>ASB Image Morphosis</h2>
    {#if version}<div class="hint">Version {version}</div>{/if}
    <p>Convert and resize many images at once.</p>
    <p class="hint">
      Free and open-source software, licensed under the GNU General Public License v3.0 or later.
      Converted images contain no metadata (EXIF, GPS, …) from the originals.
    </p>
    <div class="about-actions">
      <button class="btn" onclick={() => openUrl(REPO).catch(() => {})}>
        <Icon name="external" /> Source code
      </button>
      <button class="btn primary" onclick={() => about?.close()}>Close</button>
    </div>
  </div>
</dialog>

<style>
  .app {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    height: 100%;
  }

  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
  }

  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
    letter-spacing: 0.01em;
  }

  .spacer {
    flex: 1;
  }

  main {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 340px;
    gap: 12px;
    min-height: 0;
    padding: 0 12px 12px;
  }

  @media (max-width: 920px) {
    main {
      grid-template-columns: minmax(0, 1fr) 300px;
    }
  }

  .settings {
    display: flex;
    flex-direction: column;
    gap: 18px;
    min-height: 0;
    padding: 14px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  section h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 14px;
    font-weight: 650;
  }

  .step {
    display: inline-grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 11px;
    font-weight: 700;
  }

  .drag-overlay {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgb(91 75 240 / 14%);
    backdrop-filter: blur(2px);
    -webkit-backdrop-filter: blur(2px);
  }

  .drag-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 32px 48px;
    border: 2px dashed var(--accent);
    border-radius: 16px;
    background: var(--surface);
    color: var(--accent);
    font-size: 17px;
    font-weight: 600;
    box-shadow: var(--shadow);
  }

  dialog {
    max-width: 420px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 35%);
  }

  .about {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 24px;
    text-align: center;
  }

  .about h2 {
    margin: 6px 0 0;
    font-size: 18px;
  }

  .about p {
    margin: 4px 0;
  }

  .about-actions {
    display: flex;
    gap: 8px;
    margin-top: 10px;
  }
</style>
