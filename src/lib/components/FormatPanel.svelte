<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { FORMATS, usesQuality } from "../format";
  import { app } from "../state.svelte";

  let o = $derived(app.options);
  let info = $derived(FORMATS.find((f) => f.value === o.format));
  let focused = $state<string | null>(null);
</script>

<fieldset class="field">
  <legend class="label">Convert to</legend>
  <div class="segmented" role="radiogroup">
    {#each FORMATS as f (f.value)}
      <label class:active={o.format === f.value} class:focus={focused === f.value} title={f.hint}>
        <input
          type="radio"
          name="format"
          value={f.value}
          bind:group={app.options.format}
          onfocus={() => (focused = f.value)}
          onblur={() => (focused = null)}
        />
        {f.label}
      </label>
    {/each}
  </div>
  {#if info}<p class="hint">{info.hint}</p>{/if}
</fieldset>

{#if usesQuality(o.format, o.webpLossless) || o.format === "original"}
  <div class="field">
    <div class="split">
      <label class="label" for="quality">Quality</label>
      <span class="value">{o.quality}</span>
    </div>
    <input id="quality" type="range" min="1" max="100" bind:value={app.options.quality} />
    <div class="split hint"><span>Smaller file</span><span>Better quality</span></div>
  </div>
{/if}

{#if o.format === "webp" || o.format === "original"}
  <label class="check">
    <input type="checkbox" bind:checked={app.options.webpLossless} />
    Lossless WebP {o.format === "original" ? "(for WebP files)" : ""}
  </label>
{/if}

{#if o.format === "png" || o.format === "original"}
  <div class="field">
    <label class="label" for="png">PNG compression</label>
    <select id="png" bind:value={app.options.pngCompression}>
      <option value="fast">Fast (bigger files)</option>
      <option value="default">Balanced</option>
      <option value="best">Slow (smaller files)</option>
    </select>
  </div>
{/if}

{#if o.format === "jpeg" || o.format === "original"}
  <div class="split">
    <label class="label" for="bg">Fill transparent areas with</label>
    <input id="bg" type="color" bind:value={app.options.background} />
  </div>
{/if}

{#if o.format === "original"}
  <p class="hint">TGA, QOI, PNM, HDR and EXR files are saved as PNG.</p>
{/if}

<style>
  fieldset {
    margin: 0;
    padding: 0;
    border: none;
  }

  legend {
    margin-bottom: 6px;
    padding: 0;
  }

  p {
    margin: 0;
  }

  .split {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .value {
    min-width: 2.5em;
    font-weight: 600;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
