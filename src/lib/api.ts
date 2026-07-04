import { invoke } from "@tauri-apps/api/core";

export type ItemKind = "photo" | "video";

export interface FileEntry {
  path: string;
  file_name: string;
  size: number;
}

export interface MediaItem {
  id: string;
  kind: ItemKind;
  files: FileEntry[];
  capture_time: string;
  total_bytes: number;
  thumb_source: string | null;
}

export interface Session {
  date: string;
  items: MediaItem[];
  photo_count: number;
  video_count: number;
  total_bytes: number;
}

export interface CardInfo {
  path: string;
  name: string;
  model: string | null;
}

export interface FileResult {
  source: string;
  dest: string;
  status: "copied" | "skipped_duplicate" | "renamed" | "failed" | "cancelled";
  verified: boolean;
  error: string | null;
}

export interface ImportResult {
  files: FileResult[];
  session_dirs: string[];
  bytes_copied: number;
  cancelled: boolean;
}

export interface ImportProgress {
  file: string;
  phase: "copy" | "verify";
  file_done: number;
  file_total: number;
  overall_done: number;
  overall_total: number;
}

export interface SessionPlan {
  date: string;
  name: string;
  files: string[];
}

export interface DeleteResult {
  deleted: number;
  errors: string[];
}

export const listCards = () => invoke<CardInfo[]>("list_cards");
export const ejectVolume = (path: string) => invoke<void>("eject_volume", { path });
export const scanCard = (path: string, boundaryHour: number) =>
  invoke<Session[]>("scan_card", { path, boundaryHour });
export const makeThumb = (path: string) => invoke<string>("make_thumb", { path });
export const makePreview = (path: string) => invoke<string>("make_preview", { path });
export const checkDestination = (path: string) => invoke<void>("check_destination", { path });
export const importSessions = (plan: {
  destination: string;
  year_subfolders: boolean;
  sessions: SessionPlan[];
}) => invoke<ImportResult>("import_sessions", { plan });
export const cancelImport = () => invoke<void>("cancel_import");
export const deleteFromCard = (files: string[]) =>
  invoke<DeleteResult>("delete_from_card", { files });
export const reveal = (path: string) => invoke<void>("reveal", { path });

export function formatBytes(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(2)} GB`;
  if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(1)} MB`;
  return `${(bytes / 1e3).toFixed(0)} KB`;
}
