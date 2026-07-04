<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { checkDestination, renderPatternPreview, formatBytes } from "$lib/api";
  import { wiz, saveSettings, sessionStats, includedFiles } from "$lib/wizard.svelte";
  import { t } from "$lib/i18n.svelte";
  import PatternInput from "./PatternInput.svelte";

  let destination = $state(wiz.settings.destination);
  let checking = $state(false);
  let input = $state<HTMLInputElement | null>(null);
  let previews = $state<Map<string, string>>(new Map());
  let patternError = $state<string | null>(null);

  $effect(() => {
    input?.focus();
    input?.select();
  });

  const included = $derived(
    wiz.sessions.filter((s) => s.included && s.items.some((i) => i.included))
  );

  // Preview comes from the same Rust renderer that builds the real paths.
  $effect(() => {
    const pattern = wiz.settings.folderPattern;
    const dest = destination.replace(/\/$/, "");
    const sessions = included.map((s) => ({ date: s.date, name: s.name }));
    (async () => {
      try {
        const rendered = await Promise.all(
          sessions.map((s) => renderPatternPreview(pattern, s.date, s.name))
        );
        patternError = null;
        previews = new Map(rendered.map((p, i) => [sessions[i].date, `${dest}/${p}/`]));
      } catch (e) {
        patternError = String(e);
        previews = new Map();
      }
    })();
  });

  async function browse() {
    const dir = await open({ directory: true, defaultPath: destination });
    if (typeof dir === "string") destination = dir;
  }

  async function proceed() {
    checking = true;
    wiz.error = null;
    try {
      await checkDestination(destination);
      wiz.settings.destination = destination;
      await saveSettings();
      wiz.step = "import";
    } catch (e) {
      wiz.error = String(e);
    } finally {
      checking = false;
    }
  }
</script>

<section>
  <label class="dest">
    {t("import_to")}
    <div class="row">
      <input
        type="text"
        placeholder={t("dest_placeholder")}
        bind:value={destination}
        bind:this={input}
        onkeydown={(e) =>
          e.key === "Enter" && !e.isComposing && e.keyCode !== 229 && proceed()}
      />
      <button onclick={browse}>{t("browse")}</button>
    </div>
  </label>

  <div class="pattern-row">
    <PatternInput error={patternError} />
  </div>

  <ul class="preview">
    {#each destination.trim() ? included : [] as session (session.date)}
      {@const stats = sessionStats(session)}
      {@const path = previews.get(session.date)}
      {#if path}
        <li>
          <code>{path}</code>
          <span class="stats">
            {t("files_count", { n: includedFiles(session).length })} · {formatBytes(stats.bytes)}
          </span>
        </li>
      {/if}
    {/each}
  </ul>

  <footer>
    <button class="ghost" onclick={() => (wiz.step = "sessions")}>{t("back")}</button>
    <button
      class="primary"
      disabled={checking || !destination.trim() || patternError !== null}
      onclick={proceed}
    >
      {checking ? t("checking") : t("start_import")}
    </button>
  </footer>
</section>

<style>
  .dest {
    display: grid;
    gap: 0.4rem;
    color: var(--dim);
    font-size: 0.9rem;
  }
  .row {
    display: flex;
    gap: 0.6rem;
  }
  .row input {
    flex: 1;
    font-size: 1rem;
  }
  .pattern-row {
    margin-top: 1rem;
  }
  .preview {
    list-style: none;
    padding: 0;
    margin: 1.2rem 0;
    display: grid;
    gap: 0.5rem;
  }
  .preview li {
    background: var(--panel);
    border-radius: 10px;
    padding: 0.6rem 0.9rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }
  .preview code {
    color: var(--accent);
    font-size: 0.85rem;
    word-break: break-all;
  }
  .stats {
    color: var(--dim);
    font-size: 0.8rem;
    white-space: nowrap;
  }
  footer {
    display: flex;
    justify-content: space-between;
  }
</style>
