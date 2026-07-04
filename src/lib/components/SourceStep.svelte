<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { scanCard, type CardInfo } from "$lib/api";
  import { wiz, toUiSessions } from "$lib/wizard.svelte";
  import { t } from "$lib/i18n.svelte";

  async function useSource(card: CardInfo) {
    wiz.source = card;
    wiz.scanning = true;
    wiz.error = null;
    try {
      const sessions = await scanCard(card.path, wiz.settings.boundaryHour);
      if (sessions.length === 0) {
        wiz.error = t("no_media", { name: card.name });
        return;
      }
      wiz.sessions = toUiSessions(sessions);
      wiz.step = "sessions";
    } catch (e) {
      wiz.error = String(e);
    } finally {
      wiz.scanning = false;
    }
  }

  async function pickFolder() {
    const dir = await open({ directory: true, title: t("choose_folder") });
    if (typeof dir === "string") {
      useSource({ path: dir, name: dir.split("/").pop() ?? dir, model: null });
    }
  }

  // One card plugged in → no decision to make; scan it straight away.
  $effect(() => {
    if (wiz.cards.length === 1 && !wiz.scanning && !wiz.source) {
      useSource(wiz.cards[0]);
    }
  });
</script>

<section>
  {#if wiz.scanning}
    <p class="hint">{t("scanning", { name: wiz.source?.name ?? "" })}</p>
  {:else if wiz.cards.length === 0}
    <div class="empty">
      <p>{t("no_card")}</p>
      <p class="hint">{t("no_card_hint")}</p>
      <button onclick={pickFolder}>{t("choose_folder")}</button>
    </div>
  {:else}
    <ul class="cards">
      {#each wiz.cards as card (card.path)}
        <li>
          <button class="card" onclick={() => useSource(card)}>
            <span class="icon">💾</span>
            <span class="name">{card.name}</span>
            {#if card.model}<span class="model">{card.model}</span>{/if}
            <span class="path">{card.path}</span>
          </button>
        </li>
      {/each}
    </ul>
    <button class="ghost" onclick={pickFolder}>{t("choose_folder")}</button>
  {/if}
</section>

<style>
  .empty {
    text-align: center;
    padding: 4rem 0;
    color: var(--dim);
  }
  .hint {
    color: var(--dim);
    font-size: 0.9rem;
  }
  .cards {
    list-style: none;
    padding: 0;
    display: grid;
    gap: 0.8rem;
  }
  .card {
    width: 100%;
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 0.2rem 0.9rem;
    text-align: left;
    padding: 1rem 1.2rem;
    background: var(--panel);
    border-radius: 12px;
  }
  .card:hover {
    background: var(--panel-2);
  }
  .icon {
    grid-row: span 2;
    font-size: 1.8rem;
    align-self: center;
  }
  .name {
    font-weight: 600;
  }
  .model {
    color: var(--accent);
    font-size: 0.85rem;
    justify-self: end;
  }
  .path {
    grid-column: 2 / 4;
    color: var(--dim);
    font-size: 0.8rem;
  }
</style>
