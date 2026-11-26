<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { openPath } from "../api";
  import { formatDuration, FORMATS, RESIZE_MODES } from "../format";
  import { app } from "../state.svelte";
  import Icon from "./Icon.svelte";

  let pct = $derived(
    app.progress.total > 0 ? Math.round((app.progress.done / app.progress.total) * 100) : 0,
  );

  let plan = $derived.by(() => {
    const o = app.options;
    const fmt = FORMATS.find((f) => f.value === o.format);
    const to = o.format === "original" ? "same format" : fmt?.label;
    const r = o.resize;
    let size = "";
    switch (r.mode) {
      case "fit":
      case "fill":
      case "stretch":
        size = `${RESIZE_MODES.find((m) => m.value === r.mode)?.label.toLowerCase()} ${r.width || "auto"}×${r.height || "auto"}`;
        break;
      case "width":
        size = `width ${r.width}px`;
        break;
      case "height":
        size = `height ${r.height}px`;
        break;
      case "percent":
        size = `${r.percent}%`;
        break;
    }
    return size ? `${to}, ${size}` : `${to}`;
  });

  /** Single folder to open after converting, if there is one obvious choice. */
  let openTarget = $derived.by(() => {
    const s = app.summary;
    if (!s || s.converted === 0) return null;
    if (app.ui.saveMode === "folder" && app.ui.folder) return app.ui.folder;
    return s.outputDirs.length === 1 ? s.outputDirs[0] : null;
  });

  const n = $derived(app.files.length);

  // Brief "done" animation on the Convert button after a fully successful run.
  let done = $state(false);
  $effect(() => {
    const s = app.summary;
    if (!s || s.cancelled || s.failed > 0 || s.converted === 0) return;
    done = true;
    const t = setTimeout(() => (done = false), 1800);
    return () => {
      clearTimeout(t);
      done = false;
    };
  });
</script>

<footer class="bar">
  <div class="status" aria-live="polite">
    {#if app.running}
      <div class="progress" role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100}>
        <div class="fill" style:width="{pct}%"></div>
      </div>
      <span class="text">
        {app.cancelling ? "Stopping…" : `Converting ${app.progress.done} of ${app.progress.total}…`}
      </span>
    {:else if app.summary}
      <span class="text result" class:bad={app.summary.failed > 0}>
        <Icon name={app.summary.failed > 0 ? "alert" : "check"} />
        {app.summary.cancelled ? "Stopped. " : "Done! "}{app.summaryText(app.summary)}
        <span class="muted">in {formatDuration(app.summary.elapsedMs)}</span>
      </span>
      {#if openTarget}
        <button class="btn small" onclick={() => openTarget && openPath(openTarget).catch((e) => app.toast("error", String(e)))}>
          <Icon name="folderOpen" /> Open output folder
        </button>
      {/if}
    {:else if app.blocker}
      <span class="text muted">{app.blocker}</span>
    {:else}
      <span class="text muted">Ready: {n} image{n === 1 ? "" : "s"} → {plan}</span>
    {/if}
  </div>

  {#if app.running}
    <button class="btn stop" onclick={() => app.cancel()} disabled={app.cancelling} title="Stop (Esc)">
      <Icon name="stop" /> Cancel
    </button>
  {:else}
    <button
      class="btn primary go"
      class:done
      onclick={() => app.convert()}
      disabled={!!app.blocker}
      title={app.blocker ?? "Convert (Ctrl+Enter)"}
    >
      {#if done}
        <Icon name="check" size={18} /> Done
      {:else}
        Convert {n > 0 ? `${n} image${n === 1 ? "" : "s"}` : ""}
        <Icon name="arrowRight" />
      {/if}
    </button>
  {/if}
</footer>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
    background: var(--surface);
  }

  .status {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .text {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .muted {
    color: var(--muted);
  }

  .result {
    color: var(--success);
    font-weight: 500;
  }

  .result.bad {
    color: var(--warn);
  }

  .progress {
    flex: 0 1 320px;
    height: 8px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--surface-2);
  }

  .fill {
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    transition: width 0.2s ease-out;
  }

  .go {
    min-width: 180px;
    min-height: 40px;
    font-size: 15px;
  }

  /* ---- "done" animation ---- */
  .go.done,
  .go.done:hover:not(:disabled) {
    border-color: var(--success);
    background: var(--success);
    animation:
      pop 0.45s cubic-bezier(0.34, 1.56, 0.64, 1),
      ring 0.9s ease-out;
  }

  .go.done :global(path) {
    stroke-dasharray: 24;
    stroke-dashoffset: 24;
    animation: draw 0.35s 0.15s ease-out forwards;
  }

  @keyframes pop {
    0% {
      transform: scale(1);
    }
    40% {
      transform: scale(1.07);
    }
    100% {
      transform: scale(1);
    }
  }

  @keyframes ring {
    0% {
      box-shadow: 0 0 0 0 color-mix(in srgb, var(--success) 55%, transparent);
    }
    100% {
      box-shadow: 0 0 0 14px transparent;
    }
  }

  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .go.done,
    .go.done :global(path) {
      animation: none;
      stroke-dashoffset: 0;
    }
  }
</style>
