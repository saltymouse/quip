<script lang="ts">
  import { onMount } from "svelte";
  import {
    deleteFromCard,
    ejectVolume,
    reveal,
    formatBytes,
    type DeleteResult,
  } from "$lib/api";
  import { wiz } from "$lib/wizard.svelte";
  import { t } from "$lib/i18n.svelte";

  let deleteResult = $state<DeleteResult | null>(null);
  let deleting = $state(false);
  let ejected = $state(false);

  const copied = $derived(
    wiz.result?.files.filter((f) => f.status === "copied" || f.status === "renamed") ?? []
  );
  const skipped = $derived(
    wiz.result?.files.filter((f) => f.status === "skipped_duplicate") ?? []
  );
  const failed = $derived(
    wiz.result?.files.filter((f) => f.status === "failed" || f.status === "cancelled") ?? []
  );
  /** Only checksum-verified copies and confirmed duplicates are safe to delete. */
  const deletable = $derived(
    (wiz.result?.files ?? []).filter((f) => f.verified).map((f) => f.source)
  );

  onMount(() => {
    for (const dir of wiz.result?.session_dirs ?? []) {
      reveal(dir);
    }
  });

  async function doDelete() {
    deleting = true;
    try {
      deleteResult = await deleteFromCard(deletable);
    } catch (e) {
      wiz.error = String(e);
    } finally {
      deleting = false;
    }
  }

  async function eject() {
    if (!wiz.source) return;
    try {
      await ejectVolume(wiz.source.path);
      ejected = true;
    } catch (e) {
      wiz.error = String(e);
    }
  }

  function startOver() {
    wiz.step = "source";
    wiz.source = null;
    wiz.sessions = [];
    wiz.result = null;
    deleteResult = null;
  }
</script>

<section>
  <h2>{wiz.result?.cancelled ? t("import_cancelled") : t("import_finished")}</h2>

  <ul class="summary">
    <li class="ok">
      {t("copied_summary", {
        n: copied.length,
        size: formatBytes(wiz.result?.bytes_copied ?? 0),
      })}
    </li>
    {#if skipped.length > 0}
      <li class="warn">{t("skipped_summary", { n: skipped.length })}</li>
    {/if}
    {#if failed.length > 0}
      <li class="bad">{t("failed_summary", { n: failed.length })}</li>
    {/if}
  </ul>

  {#if failed.length > 0}
    <details>
      <summary>{t("failures")}</summary>
      <ul class="failures">
        {#each failed as f (f.source)}
          <li><code>{f.source}</code> — {f.error ?? f.status}</li>
        {/each}
      </ul>
    </details>
  {/if}

  <div class="folders">
    {#each wiz.result?.session_dirs ?? [] as dir (dir)}
      <button onclick={() => reveal(dir)}>📂 {dir.split("/").pop()}</button>
    {/each}
  </div>

  <hr />

  {#if deleteResult}
    <p class="ok">
      {t("deleted_summary", { n: deleteResult.deleted })}
      {#if deleteResult.errors.length > 0}
        <span class="bad">{t("delete_errors", { n: deleteResult.errors.length })}</span>
      {/if}
    </p>
    <button class="primary" disabled={ejected} onclick={eject}>
      {ejected ? t("ejected") : t("eject")}
    </button>
  {:else if deletable.length > 0 && !wiz.result?.cancelled}
    <p>{t("delete_prompt", { n: deletable.length, name: wiz.source?.name ?? "" })}</p>
    <div class="row">
      <button class="primary" onclick={startOver}>{t("keep_files")}</button>
      <button class="danger" disabled={deleting} onclick={doDelete}>
        {deleting ? t("deleting") : t("delete_files", { n: deletable.length })}
      </button>
    </div>
  {/if}

  <footer>
    <button class="ghost" onclick={startOver}>{t("import_another")}</button>
  </footer>
</section>

<style>
  .summary {
    list-style: none;
    padding: 0;
    display: grid;
    gap: 0.4rem;
  }
  .ok {
    color: var(--ok);
  }
  .warn {
    color: var(--warn);
  }
  .bad {
    color: var(--bad);
  }
  .failures code {
    font-size: 0.8rem;
  }
  .folders {
    display: flex;
    flex-wrap: wrap;
    gap: 0.6rem;
    margin: 1rem 0;
  }
  hr {
    border: none;
    border-top: 1px solid var(--panel-2);
    margin: 1.5rem 0;
  }
  .row {
    display: flex;
    gap: 0.8rem;
  }
  footer {
    margin-top: 2.5rem;
  }
</style>
