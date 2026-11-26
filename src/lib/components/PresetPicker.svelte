<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { app } from "../state.svelte";
  import type { Preset } from "../types";
  import Icon from "./Icon.svelte";

  let saving = $state(false);
  let newName = $state("");

  const settingsKey = (s: Preset["settings"]) =>
    JSON.stringify([s.format, s.quality, s.webpLossless, s.pngCompression, s.background, s.resize, s.noUpscale]);

  /** The dropdown follows the settings: it shows the preset they match, or "Custom" once edited. */
  let current = $derived(app.presets.find((p) => settingsKey(p.settings) === settingsKey(app.options)));
  let chosen = $derived(current?.id ?? "");

  function apply(id: string) {
    const preset = app.presets.find((p) => p.id === id);
    if (preset) app.applyPreset(preset);
  }

  function save() {
    const name = newName.trim();
    if (!name) return;
    app.savePreset(name);
    app.toast("success", `Saved preset "${name}"`, undefined, 2500);
    newName = "";
    saving = false;
  }
</script>

<div class="field">
  <label class="label" for="preset">Preset</label>
  {#if saving}
    <form
      class="row"
      onsubmit={(e) => {
        e.preventDefault();
        save();
      }}
    >
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="text"
        placeholder="Preset name"
        bind:value={newName}
        autofocus
        onkeydown={(e) => e.key === "Escape" && (saving = false)}
      />
      <button class="btn primary small" type="submit" disabled={!newName.trim()}>Save</button>
      <button class="btn ghost small" type="button" onclick={() => (saving = false)}>Cancel</button>
    </form>
  {:else}
    <div class="row">
      <select id="preset" value={chosen} onchange={(e) => apply(e.currentTarget.value)}>
        <option value="" disabled>Custom settings</option>
        <optgroup label="Built-in">
          {#each app.presets.filter((p) => p.builtIn) as p (p.id)}
            <option value={p.id}>{p.name}</option>
          {/each}
        </optgroup>
        {#if app.userPresets.length > 0}
          <optgroup label="My presets">
            {#each app.userPresets as p (p.id)}
              <option value={p.id}>{p.name}</option>
            {/each}
          </optgroup>
        {/if}
      </select>
      <button class="icon-btn" title="Save current settings as a preset" onclick={() => (saving = true)}>
        <Icon name="save" />
      </button>
      {#if current && !current.builtIn}
        <button
          class="icon-btn"
          title="Delete this preset"
          onclick={() => current && app.deletePreset(current.id)}
        >
          <Icon name="trash" />
        </button>
      {/if}
      <button class="icon-btn" title="Reset to default settings" onclick={() => app.resetSettings()}>
        <Icon name="reset" />
      </button>
    </div>
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .row select,
  .row input {
    flex: 1;
    min-width: 0;
  }
</style>
