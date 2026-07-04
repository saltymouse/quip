<script lang="ts">
  import { wiz, saveSettings } from "$lib/wizard.svelte";
  import { t } from "$lib/i18n.svelte";

  let { error = null }: { error?: string | null } = $props();
  let input = $state<HTMLInputElement | null>(null);

  const TOKENS = ["{year}", "{month}", "{day}", "{date}", "{name}"] as const;

  function insert(token: string) {
    const el = input;
    if (!el) return;
    const start = el.selectionStart ?? wiz.settings.folderPattern.length;
    const end = el.selectionEnd ?? start;
    wiz.settings.folderPattern =
      wiz.settings.folderPattern.slice(0, start) +
      token +
      wiz.settings.folderPattern.slice(end);
    saveSettings();
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(start + token.length, start + token.length);
    });
  }
</script>

<div class="pattern">
  <label>
    {t("folder_pattern")}
    <input
      type="text"
      spellcheck="false"
      bind:this={input}
      bind:value={wiz.settings.folderPattern}
      onchange={saveSettings}
    />
  </label>
  <div class="pills">
    {#each TOKENS as token (token)}
      <button class="pill" title={t("insert_variable")} onclick={() => insert(token)}>
        {token}
      </button>
    {/each}
  </div>
  {#if error}
    <p class="err">{error}</p>
  {/if}
</div>

<style>
  .pattern {
    display: grid;
    gap: 0.45rem;
  }
  label {
    display: grid;
    gap: 0.3rem;
    font-size: 0.85rem;
    color: var(--dim);
  }
  input {
    font-family: ui-monospace, "SF Mono", monospace;
    font-size: 0.9rem;
  }
  .pills {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  .pill {
    font-family: ui-monospace, "SF Mono", monospace;
    font-size: 0.75rem;
    padding: 0.2em 0.65em;
    border-radius: 999px;
    background: var(--panel-2);
    border: 1px solid #333a47;
    color: var(--accent);
  }
  .pill:hover {
    border-color: var(--accent);
  }
  .err {
    color: var(--bad);
    font-size: 0.8rem;
    margin: 0;
  }
</style>
