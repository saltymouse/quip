<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { listCards, type CardInfo } from "$lib/api";
  import { wiz, loadSettings, saveSettings } from "$lib/wizard.svelte";
  import { t, type MsgKey } from "$lib/i18n.svelte";
  import SourceStep from "$lib/components/SourceStep.svelte";
  import SessionsStep from "$lib/components/SessionsStep.svelte";
  import DestinationStep from "$lib/components/DestinationStep.svelte";
  import ImportStep from "$lib/components/ImportStep.svelte";
  import DoneStep from "$lib/components/DoneStep.svelte";
  import PatternInput from "$lib/components/PatternInput.svelte";

  const steps: { id: typeof wiz.step; label: MsgKey }[] = [
    { id: "source", label: "step_card" },
    { id: "sessions", label: "step_sessions" },
    { id: "destination", label: "step_destination" },
    { id: "import", label: "step_import" },
    { id: "done", label: "step_done" },
  ];

  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      await loadSettings();
      wiz.cards = await listCards();
      unlisten = await listen<CardInfo[]>("cards-changed", (event) => {
        wiz.cards = event.payload;
        if (
          wiz.step === "source" &&
          wiz.source &&
          !event.payload.some((c) => c.path === wiz.source?.path)
        ) {
          wiz.source = null;
        }
      });
    })();
    return () => unlisten?.();
  });
</script>

<main>
  <header>
    <h1>quip</h1>
    <nav>
      {#each steps as s, i (s.id)}
        <span
          class="crumb"
          class:active={wiz.step === s.id}
          class:past={steps.findIndex((x) => x.id === wiz.step) > i}
        >
          {t(s.label)}
        </span>
      {/each}
    </nav>
    <button
      class="ghost gear"
      title={t("settings")}
      onclick={() => (wiz.settingsOpen = !wiz.settingsOpen)}>⚙</button
    >
  </header>

  {#if wiz.settingsOpen}
    <section class="settings">
      <label>
        {t("default_destination")}
        <input
          type="text"
          placeholder={t("dest_placeholder")}
          bind:value={wiz.settings.destination}
          onchange={saveSettings}
        />
      </label>
      <label>
        {t("day_boundary")}
        <input
          type="number"
          min="0"
          max="12"
          bind:value={wiz.settings.boundaryHour}
          onchange={saveSettings}
        />
      </label>
      <PatternInput />
      <label>
        {t("language")}
        <select bind:value={wiz.settings.language} onchange={saveSettings}>
          <option value="system">{t("lang_system")}</option>
          <option value="en">English</option>
          <option value="ja">日本語</option>
        </select>
      </label>
    </section>
  {/if}

  {#if wiz.error}
    <div class="error" role="alert">
      {wiz.error}
      <button class="ghost" onclick={() => (wiz.error = null)}>✕</button>
    </div>
  {/if}

  {#if wiz.step === "source"}
    <SourceStep />
  {:else if wiz.step === "sessions"}
    <SessionsStep />
  {:else if wiz.step === "destination"}
    <DestinationStep />
  {:else if wiz.step === "import"}
    <ImportStep />
  {:else if wiz.step === "done"}
    <DoneStep />
  {/if}
</main>

<style>
  :global(:root) {
    color-scheme: dark;
    --bg: #16181d;
    --panel: #1f232b;
    --panel-2: #262b35;
    --text: #e6e9ef;
    --dim: #9aa3b2;
    --accent: #6aa1ff;
    --ok: #5fd38a;
    --warn: #f0b35e;
    --bad: #f2748c;
    font-family: -apple-system, "Hiragino Sans", "Helvetica Neue", sans-serif;
  }
  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--text);
  }
  :global(button) {
    font: inherit;
    border: none;
    border-radius: 8px;
    padding: 0.55em 1.1em;
    background: var(--panel-2);
    color: var(--text);
    cursor: pointer;
  }
  :global(button.primary) {
    background: var(--accent);
    color: #10131a;
    font-weight: 600;
  }
  :global(button.danger) {
    background: var(--bad);
    color: #10131a;
    font-weight: 600;
  }
  :global(button.ghost) {
    background: transparent;
    color: var(--dim);
  }
  :global(button:disabled) {
    opacity: 0.45;
    cursor: default;
  }
  :global(input[type="text"]),
  :global(input[type="number"]),
  :global(select) {
    font: inherit;
    background: var(--panel-2);
    border: 1px solid #333a47;
    border-radius: 8px;
    color: var(--text);
    padding: 0.45em 0.7em;
  }
  :global(input:focus) {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }

  main {
    max-width: 1000px;
    margin: 0 auto;
    padding: 1rem 1.5rem 3rem;
  }
  header {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    padding: 0.5rem 0 1rem;
  }
  h1 {
    font-size: 1.3rem;
    margin: 0;
    letter-spacing: 0.04em;
  }
  nav {
    display: flex;
    gap: 0.4rem;
    flex: 1;
  }
  .crumb {
    color: var(--dim);
    font-size: 0.85rem;
    padding: 0.25em 0.7em;
    border-radius: 999px;
  }
  .crumb.past {
    color: var(--ok);
  }
  .crumb.active {
    background: var(--panel-2);
    color: var(--text);
  }
  .gear {
    font-size: 1.1rem;
  }
  .settings {
    background: var(--panel);
    border-radius: 12px;
    padding: 1rem 1.2rem;
    margin-bottom: 1rem;
    display: grid;
    gap: 0.8rem;
  }
  .settings label {
    display: grid;
    gap: 0.3rem;
    font-size: 0.85rem;
    color: var(--dim);
  }
  .error {
    background: #3a2230;
    border: 1px solid var(--bad);
    border-radius: 10px;
    padding: 0.7rem 1rem;
    margin-bottom: 1rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
</style>
