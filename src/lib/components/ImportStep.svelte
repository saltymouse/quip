<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { listen } from "@tauri-apps/api/event";
  import {
    importSessions,
    cancelImport,
    formatBytes,
    type ImportProgress,
  } from "$lib/api";
  import { wiz, includedFiles } from "$lib/wizard.svelte";
  import { t } from "$lib/i18n.svelte";

  let progress = $state<ImportProgress | null>(null);
  let mbps = $state(0);
  let cancelling = $state(false);

  // Thumbnails were already built on the Sessions screen; map every file name
  // (ARW, JPG, MP4) of an item to that item's thumb so the current file can
  // be shown while it copies.
  const thumbByFile = new Map<string, { url: string; video: boolean }>();
  for (const session of wiz.sessions) {
    for (const item of session.items) {
      if (!item.thumb || item.thumb === "none") continue;
      for (const file of item.files) {
        thumbByFile.set(file.file_name, {
          url: item.thumb,
          video: item.kind === "video",
        });
      }
    }
  }
  const current = $derived(progress ? thumbByFile.get(progress.file) : undefined);

  let lastBytes = 0;
  let lastTime = 0;

  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      unlisten = await listen<ImportProgress>("import-progress", (event) => {
        progress = event.payload;
        const now = performance.now();
        if (lastTime > 0 && event.payload.overall_done > lastBytes) {
          const dt = (now - lastTime) / 1000;
          if (dt > 0.3) {
            mbps = (event.payload.overall_done - lastBytes) / dt / 1e6;
            lastBytes = event.payload.overall_done;
            lastTime = now;
          }
        } else {
          lastBytes = event.payload.overall_done;
          lastTime = now;
        }
      });

      try {
        wiz.result = await importSessions({
          destination: wiz.settings.destination,
          folder_pattern: wiz.settings.folderPattern,
          sessions: wiz.sessions
            .filter((s) => s.included && s.items.some((i) => i.included))
            .map((s) => ({
              date: s.date,
              name: s.name,
              files: includedFiles(s),
            })),
        });
        wiz.step = "done";
      } catch (e) {
        wiz.error = String(e);
        wiz.step = "destination";
      }
    })();
    return () => unlisten?.();
  });

  const pct = $derived(
    progress && progress.overall_total > 0
      ? (progress.overall_done / progress.overall_total) * 100
      : 0
  );
</script>

<section>
  <h2>{t("importing")}</h2>
  <div class="stage">
    {#key current?.url}
      <div class="shot" in:fade={{ duration: 250 }}>
        {#if current}
          <img src={current.url} alt={progress?.file} />
          {#if current.video}<span class="play">▶</span>{/if}
        {:else}
          <span class="placeholder">{progress ? "🎞" : "…"}</span>
        {/if}
      </div>
    {/key}
  </div>
  <div class="bar">
    <div class="fill" style={`width:${pct}%`}></div>
  </div>
  {#if progress}
    <p class="detail">
      <span class="file">{progress.file}</span>
      <span class="phase" class:verify={progress.phase === "verify"}>
        {progress.phase === "verify" ? t("verifying") : t("copying")}
      </span>
    </p>
    <p class="numbers">
      {formatBytes(progress.overall_done)} / {formatBytes(progress.overall_total)}
      {#if mbps > 0}· {mbps.toFixed(0)} MB/s{/if}
    </p>
  {:else}
    <p class="detail">{t("preparing")}</p>
  {/if}
  <footer>
    <button
      class="danger"
      disabled={cancelling}
      onclick={() => {
        cancelling = true;
        cancelImport();
      }}
    >
      {cancelling ? t("cancelling") : t("cancel")}
    </button>
  </footer>
</section>

<style>
  section {
    padding-top: 1.5rem;
    text-align: center;
  }
  .stage {
    display: grid;
    place-items: center;
    margin-top: 1.2rem;
  }
  .shot {
    grid-area: 1 / 1;
    position: relative;
    height: 220px;
    aspect-ratio: 3 / 2;
    border-radius: 12px;
    overflow: hidden;
    background: var(--panel);
    display: grid;
    place-items: center;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.45);
  }
  .shot img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .play {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 2rem;
    color: rgba(255, 255, 255, 0.85);
    text-shadow: 0 2px 8px rgba(0, 0, 0, 0.7);
  }
  .placeholder {
    font-size: 2.5rem;
    color: var(--dim);
  }
  .bar {
    height: 14px;
    background: var(--panel-2);
    border-radius: 999px;
    overflow: hidden;
    margin: 1.5rem 0 1rem;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s linear;
  }
  .detail {
    display: flex;
    justify-content: center;
    gap: 0.8rem;
    align-items: baseline;
  }
  .file {
    font-variant-numeric: tabular-nums;
  }
  .phase {
    color: var(--dim);
    font-size: 0.8rem;
  }
  .phase.verify {
    color: var(--warn);
  }
  .numbers {
    color: var(--dim);
    font-variant-numeric: tabular-nums;
  }
  footer {
    margin-top: 2rem;
  }
</style>
