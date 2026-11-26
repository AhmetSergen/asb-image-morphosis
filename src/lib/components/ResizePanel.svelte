<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { RESIZE_MODES } from "../format";
  import { app } from "../state.svelte";

  let r = $derived(app.options.resize);
  let hasW = $derived("width" in r);
  let hasH = $derived("height" in r);
  let box = $derived(r.mode === "fit" || r.mode === "fill" || r.mode === "stretch");

  const HINTS: Record<string, string> = {
    none: "Images keep their size.",
    fit: "Keeps proportions. The image fits inside the box. Leave a side at 0 to ignore it.",
    width: "Keeps proportions. The height follows the width.",
    height: "Keeps proportions. The width follows the height.",
    percent: "Keeps proportions. Scales both sides by the same amount.",
    fill: "Fills the box exactly and crops what sticks out (centred).",
    stretch: "Forces the exact size. Images may look squashed.",
  };

  const COMMON = [640, 1280, 1920, 2560, 3840];
  const PERCENTS = [25, 50, 75];

  const num = (e: Event) => (e.currentTarget as HTMLInputElement).valueAsNumber;
  const dimValue = (key: "width" | "height") =>
    key in r ? (r as unknown as Record<string, number>)[key] : 0;
</script>

<div class="field">
  <label class="label" for="resize">Resize</label>
  <select id="resize" value={r.mode} onchange={(e) => app.setResizeMode(e.currentTarget.value as never)}>
    {#each RESIZE_MODES as m (m.value)}
      <option value={m.value}>{m.label}</option>
    {/each}
  </select>
</div>

{#if hasW || hasH}
  <div class="dims" class:single={!box}>
    {#if hasW}
      <label class="dim">
        <span class="label">Width</span>
        <span class="unit-input">
          <input type="number" min="0" max="65535" value={dimValue("width") || ""} placeholder="auto" oninput={(e) => app.setDim("width", num(e))} />
          <span class="unit">px</span>
        </span>
      </label>
    {/if}
    {#if box}<span class="times" aria-hidden="true">×</span>{/if}
    {#if hasH}
      <label class="dim">
        <span class="label">Height</span>
        <span class="unit-input">
          <input type="number" min="0" max="65535" value={dimValue("height") || ""} placeholder="auto" oninput={(e) => app.setDim("height", num(e))} />
          <span class="unit">px</span>
        </span>
      </label>
    {/if}
  </div>
  {#if r.mode === "fit" || r.mode === "width" || r.mode === "height"}
    <div class="chips" aria-label="Common sizes">
      {#each COMMON as px (px)}
        <button
          class="btn small ghost"
          onclick={() => {
            app.setDim("width", px);
            app.setDim("height", px);
          }}>{px}</button
        >
      {/each}
    </div>
  {/if}
{/if}

{#if r.mode === "percent"}
  <label class="dim">
    <span class="label">Scale</span>
    <span class="unit-input">
      <input type="number" min="1" max="1000" value={r.percent || ""} oninput={(e) => app.setDim("percent", num(e))} />
      <span class="unit">%</span>
    </span>
  </label>
  <div class="chips">
    {#each PERCENTS as p (p)}
      <button class="btn small ghost" onclick={() => app.setDim("percent", p)}>{p}%</button>
    {/each}
  </div>
{/if}

<p class="hint">{HINTS[r.mode]}</p>

{#if r.mode !== "none"}
  <label class="check">
    <input type="checkbox" bind:checked={app.options.noUpscale} />
    Don't enlarge smaller images
  </label>
{/if}

<style>
  p {
    margin: 0;
  }

  .dims {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: end;
    gap: 8px;
  }

  .dims.single {
    grid-template-columns: 1fr;
  }

  .dim {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .times {
    padding-bottom: 8px;
    color: var(--muted);
  }

  .unit-input {
    position: relative;
    display: block;
  }

  .unit-input input {
    padding-right: 30px;
  }

  .unit {
    position: absolute;
    top: 50%;
    right: 10px;
    transform: translateY(-50%);
    font-size: 12px;
    color: var(--muted);
    pointer-events: none;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    margin-top: -4px;
  }

  .chips .btn {
    min-height: 24px;
    padding: 0 8px;
    font-size: 12px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
</style>
