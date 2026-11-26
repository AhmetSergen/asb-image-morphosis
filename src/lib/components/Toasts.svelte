<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script lang="ts">
  import { fly } from "svelte/transition";
  import { app } from "../state.svelte";
  import Icon from "./Icon.svelte";
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}" transition:fly={{ y: 12, duration: 180 }} role={t.kind === "error" ? "alert" : "status"}>
      <span class="icon"><Icon name={t.kind === "error" ? "alert" : t.kind === "success" ? "check" : "info"} /></span>
      <div class="body">
        <div class="title">{t.text}</div>
        {#if t.detail?.length}
          <ul>
            {#each t.detail as line, i (i)}<li>{line}</li>{/each}
          </ul>
        {/if}
      </div>
      <button class="icon-btn" aria-label="Dismiss" onclick={() => app.dismiss(t.id)}><Icon name="x" size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 76px;
    z-index: 50;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: min(380px, calc(100vw - 32px));
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 8px 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow);
    pointer-events: auto;
  }

  .icon {
    padding-top: 2px;
  }

  .success .icon {
    color: var(--success);
  }

  .error .icon {
    color: var(--danger);
  }

  .info .icon {
    color: var(--accent);
  }

  .body {
    flex: 1;
    min-width: 0;
  }

  .title {
    font-weight: 600;
  }

  ul {
    margin: 4px 0 0;
    padding-left: 16px;
    font-size: 12px;
    color: var(--muted);
  }

  li {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
