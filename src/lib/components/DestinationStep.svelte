<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { checkDestination, formatBytes } from "$lib/api";
  import { wiz, saveSettings, sessionStats, includedFiles } from "$lib/wizard.svelte";
  import { t } from "$lib/i18n.svelte";

  let destination = $state(wiz.settings.destination);
  let checking = $state(false);
  let input = $state<HTMLInputElement | null>(null);

  $effect(() => {
    input?.focus();
    input?.select();
  });

  const included = $derived(
    wiz.sessions.filter((s) => s.included && s.items.some((i) => i.included))
  );

  function folderName(date: string, name: string): string {
    const clean = name.trim().replace(/[/:]/g, "-");
    return clean ? `${date} ${clean}` : date;
  }

  function fullPath(date: string, name: string): string {
    const year = wiz.settings.yearSubfolders ? `${date.slice(0, 4)}/` : "";
    return `${destination.replace(/\/$/, "")}/${year}${folderName(date, name)}/`;
  }

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

  <ul class="preview">
    {#each destination.trim() ? included : [] as session (session.date)}
      {@const stats = sessionStats(session)}
      <li>
        <code>{fullPath(session.date, session.name)}</code>
        <span class="stats">
          {t("files_count", { n: includedFiles(session).length })} · {formatBytes(stats.bytes)}
        </span>
      </li>
    {/each}
  </ul>

  <footer>
    <button class="ghost" onclick={() => (wiz.step = "sessions")}>{t("back")}</button>
    <button class="primary" disabled={checking || !destination.trim()} onclick={proceed}>
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
