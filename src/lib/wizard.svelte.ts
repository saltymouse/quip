import { load, type Store } from "@tauri-apps/plugin-store";
import type { CardInfo, ImportResult, MediaItem, Session } from "./api";

export type Step = "source" | "sessions" | "destination" | "import" | "done";

export interface UiItem extends MediaItem {
  included: boolean;
  thumb: string | null;
}

export interface UiSession {
  date: string;
  name: string;
  included: boolean;
  items: UiItem[];
}

export interface Settings {
  destination: string;
  boundaryHour: number;
  folderPattern: string;
  language: "system" | "en" | "ja";
}

export const DEFAULT_PATTERN = "{year}/{date} {name}";

const DEFAULTS: Settings = {
  destination: "",
  boundaryHour: 4,
  folderPattern: DEFAULT_PATTERN,
  language: "system",
};

export const wiz = $state({
  step: "source" as Step,
  cards: [] as CardInfo[],
  source: null as CardInfo | null,
  scanning: false,
  sessions: [] as UiSession[],
  settings: { ...DEFAULTS } as Settings,
  settingsOpen: false,
  result: null as ImportResult | null,
  error: null as string | null,
});

let store: Store | null = null;

export async function loadSettings() {
  store = await load("settings.json", { autoSave: true, defaults: {} });
  const saved = await store.get<Settings & { yearSubfolders?: boolean }>("settings");
  if (saved) {
    wiz.settings = { ...DEFAULTS, ...saved };
    // Settings written before folder patterns existed had a yearSubfolders flag.
    if (!saved.folderPattern) {
      wiz.settings.folderPattern =
        saved.yearSubfolders === false ? "{date} {name}" : DEFAULT_PATTERN;
    }
  }
}

export async function saveSettings() {
  await store?.set("settings", $state.snapshot(wiz.settings));
}

export function toUiSessions(sessions: Session[]): UiSession[] {
  return sessions.map((s) => ({
    date: s.date,
    name: "",
    included: true,
    items: s.items.map((i) => ({ ...i, included: true, thumb: null })),
  }));
}

/** Merge session at `index` into the one above it (keeps the earlier date). */
export function mergeUp(index: number) {
  if (index <= 0) return;
  const [removed] = wiz.sessions.splice(index, 1);
  wiz.sessions[index - 1].items.push(...removed.items);
  if (!wiz.sessions[index - 1].name && removed.name) {
    wiz.sessions[index - 1].name = removed.name;
  }
}

export function includedFiles(session: UiSession): string[] {
  return session.items
    .filter((i) => i.included)
    .flatMap((i) => i.files.map((f) => f.path));
}

export function sessionStats(session: UiSession) {
  const items = session.items.filter((i) => i.included);
  return {
    photos: items.filter((i) => i.kind === "photo").length,
    videos: items.filter((i) => i.kind === "video").length,
    bytes: items.reduce((sum, i) => sum + i.total_bytes, 0),
  };
}
