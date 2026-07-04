<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { makeThumb, makePreview, formatBytes } from "$lib/api";
  import { wiz, mergeUp, sessionStats } from "$lib/wizard.svelte";
  import { t } from "$lib/i18n.svelte";

  let lightbox = $state<string | null>(null);
  let lightboxLoading = $state(false);

  // Fill in thumbnails a few at a time so the grid streams in.
  $effect(() => {
    const pending = wiz.sessions
      .flatMap((s) => s.items)
      .filter((i) => i.thumb === null && i.thumb_source);
    if (pending.length === 0) return;
    let stopped = false;
    const workers = Array.from({ length: 4 }, async () => {
      while (!stopped && pending.length > 0) {
        const item = pending.shift()!;
        try {
          const cached = await makeThumb(item.thumb_source!);
          item.thumb = convertFileSrc(cached);
        } catch {
          item.thumb = "none";
        }
      }
    });
    void workers;
    return () => {
      stopped = true;
    };
  });

  async function openLightbox(source: string) {
    lightboxLoading = true;
    try {
      lightbox = convertFileSrc(await makePreview(source));
    } catch {
      lightbox = null;
    } finally {
      lightboxLoading = false;
    }
  }

  function onNameKeydown(event: KeyboardEvent, index: number) {
    if (event.key !== "Enter") return;
    // Enter that commits an IME conversion (Japanese input) must not navigate.
    if (event.isComposing || event.keyCode === 229) return;
    event.preventDefault();
    const inputs = document.querySelectorAll<HTMLInputElement>("input.session-name");
    if (index + 1 < inputs.length) {
      inputs[index + 1].focus();
    } else {
      wiz.step = "destination";
    }
  }

  const anyIncluded = $derived(
    wiz.sessions.some((s) => s.included && s.items.some((i) => i.included))
  );
  const excludedCount = $derived(
    wiz.sessions
      .filter((s) => s.included)
      .reduce((sum, s) => sum + s.items.filter((i) => !i.included).length, 0)
  );
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape") lightbox = null;
  }}
/>

<section>
  <p class="hint">{t("sessions_hint")}</p>

  {#each wiz.sessions as session, si (session.date + si)}
    {@const stats = sessionStats(session)}
    <article class="session" class:excluded={!session.included}>
      <header>
        <input type="checkbox" bind:checked={session.included} title={t("import_this_session")} />
        <span class="date">{session.date}</span>
        <input
          class="session-name"
          type="text"
          placeholder={t("session_name_placeholder")}
          bind:value={session.name}
          disabled={!session.included}
          onkeydown={(e) => onNameKeydown(e, si)}
        />
        <span class="stats">
          {t("stats", {
            photos: stats.photos,
            videos: stats.videos,
            size: formatBytes(stats.bytes),
          })}
        </span>
        {#if si > 0}
          <button class="ghost" title={t("merge_title")} onclick={() => mergeUp(si)}>
            {t("merge")}
          </button>
        {/if}
      </header>
      <div class="grid">
        {#each session.items as item (item.id)}
          <div class="cell" class:off={!item.included}>
            <button
              class="thumb"
              title={`${item.id} — ${item.capture_time.replace("T", " ")}`}
              onclick={() => item.thumb_source && openLightbox(item.thumb_source)}
            >
              {#if item.thumb && item.thumb !== "none"}
                <img src={item.thumb} alt={item.id} loading="lazy" />
              {:else}
                <span class="placeholder">{item.kind === "video" ? "🎬" : "…"}</span>
              {/if}
              {#if item.kind === "video"}<span class="badge">▶</span>{/if}
            </button>
            <button
              class="exclude"
              title={item.included ? t("exclude_item") : t("include_item")}
              onclick={() => (item.included = !item.included)}
            >
              {item.included ? "✕" : "↩"}
            </button>
          </div>
        {/each}
      </div>
    </article>
  {/each}

  <footer>
    <button class="ghost" onclick={() => (wiz.step = "source")}>{t("back")}</button>
    <div class="footer-right">
      {#if excludedCount > 0}
        <span class="excluded-warning">⚠ {t("excluded_warning", { n: excludedCount })}</span>
      {/if}
      <button
        class="primary"
        disabled={!anyIncluded}
        onclick={() => (wiz.step = "destination")}>{t("continue")}</button
      >
    </div>
  </footer>
</section>

{#if lightbox || lightboxLoading}
  <button class="lightbox" onclick={() => (lightbox = null)} aria-label="Close preview">
    {#if lightbox}<img src={lightbox} alt="preview" />{:else}<span>loading…</span>{/if}
  </button>
{/if}

<style>
  .hint {
    color: var(--dim);
    font-size: 0.9rem;
  }
  .session {
    background: var(--panel);
    border-radius: 12px;
    padding: 0.9rem 1rem;
    margin-bottom: 1rem;
  }
  .session.excluded {
    opacity: 0.45;
  }
  .session > header {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    margin-bottom: 0.7rem;
  }
  .date {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  input.session-name {
    flex: 1;
  }
  .stats {
    color: var(--dim);
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    gap: 6px;
  }
  .cell {
    position: relative;
  }
  .cell.off .thumb {
    opacity: 0.3;
    outline: 2px solid var(--bad);
    outline-offset: -2px;
  }
  .thumb {
    width: 100%;
    aspect-ratio: 3 / 2;
    padding: 0;
    border-radius: 6px;
    overflow: hidden;
    background: var(--panel-2);
    display: block;
    position: relative;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .placeholder {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--dim);
  }
  .badge {
    position: absolute;
    left: 4px;
    bottom: 4px;
    background: rgba(0, 0, 0, 0.65);
    border-radius: 4px;
    font-size: 0.65rem;
    padding: 1px 5px;
  }
  .exclude {
    position: absolute;
    top: 4px;
    right: 4px;
    padding: 2px 7px;
    font-size: 0.75rem;
    line-height: 1.4;
    background: rgba(0, 0, 0, 0.6);
    color: #fff;
    border-radius: 4px;
    opacity: 0;
  }
  .cell:hover .exclude {
    opacity: 1;
  }
  .exclude:hover {
    background: var(--bad);
    color: #10131a;
  }
  /* Excluded shots keep their restore button visible so they can't be missed. */
  .cell.off .exclude {
    opacity: 1;
    background: var(--ok);
    color: #10131a;
  }
  footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 1rem;
  }
  .footer-right {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .excluded-warning {
    color: var(--warn);
    font-size: 0.9rem;
  }
  .lightbox {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.85);
    display: grid;
    place-items: center;
    border-radius: 0;
    z-index: 10;
  }
  .lightbox img {
    max-width: 92vw;
    max-height: 92vh;
    border-radius: 8px;
  }
</style>
