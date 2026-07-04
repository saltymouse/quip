import { wiz } from "./wizard.svelte";

export type Lang = "en" | "ja";
export type LangPref = "system" | Lang;

const en = {
  step_card: "Card",
  step_sessions: "Sessions",
  step_destination: "Destination",
  step_import: "Import",
  step_done: "Done",
  settings: "Settings",
  default_destination: "Default destination",
  day_boundary: "Day boundary (shots before this hour count as the previous day)",
  year_folders: "Group into year folders (2026/2026-05-29 …)",
  language: "Language",
  lang_system: "System",
  scanning: "Scanning {name}…",
  no_card: "No camera card detected.",
  no_card_hint:
    "Plug in an SD card or connect the camera by USB (Mass Storage mode) — it will appear here automatically.",
  choose_folder: "Choose folder manually…",
  no_media: "No photos or videos found on {name}",
  sessions_hint:
    "Name each session (Enter to accept and move on — a name is optional). Click a photo to preview it, ✕ to exclude it from the import.",
  exclude_item: "Exclude from import",
  include_item: "Put back into import",
  excluded_warning: "{n} excluded from this import",
  session_name_placeholder: "session name (e.g. 奥津宮神社)",
  import_this_session: "Import this session",
  stats: "{photos} photos · {videos} videos · {size}",
  merge: "⤴ merge",
  merge_title: "Merge into the session above",
  preview: "Preview",
  back: "← Back",
  continue: "Continue →",
  import_to: "Import to",
  dest_placeholder: "your photo archive folder — pick one with Browse…",
  browse: "Browse…",
  start_import: "Start import →",
  checking: "Checking…",
  files_count: "{n} files",
  importing: "Importing…",
  preparing: "Preparing…",
  copying: "copying",
  verifying: "verifying checksum",
  cancel: "Cancel",
  cancelling: "Cancelling…",
  import_finished: "Import finished",
  import_cancelled: "Import cancelled",
  copied_summary: "✓ {n} files copied & verified ({size})",
  skipped_summary: "⏭ {n} already in the archive (skipped)",
  failed_summary: "✕ {n} failed / not copied — kept on card",
  failures: "Failures",
  delete_prompt:
    "Delete the {n} verified files from {name}? Failed or excluded files stay untouched.",
  keep_files: "Keep files on card",
  delete_files: "Delete {n} files from card",
  deleting: "Deleting…",
  deleted_summary: "🧹 Deleted {n} files from the card.",
  delete_errors: "{n} could not be removed.",
  eject: "⏏ Eject card",
  ejected: "✓ Ejected — safe to unplug",
  import_another: "Import another card",
};

const ja: typeof en = {
  step_card: "カード",
  step_sessions: "セッション",
  step_destination: "保存先",
  step_import: "取り込み",
  step_done: "完了",
  settings: "設定",
  default_destination: "デフォルトの保存先",
  day_boundary: "日付の区切り時刻（この時刻より前の撮影は前日の扱い）",
  year_folders: "年ごとのフォルダにまとめる（2026/2026-05-29 …）",
  language: "言語",
  lang_system: "システム",
  scanning: "{name} をスキャン中…",
  no_card: "カメラのカードが見つかりません。",
  no_card_hint:
    "SDカードを挿すか、カメラをUSB（マスストレージモード）で接続してください。自動的にここに表示されます。",
  choose_folder: "フォルダを手動で選択…",
  no_media: "{name} に写真・動画が見つかりません",
  sessions_hint:
    "各セッションに名前を付けてください（Enterで確定して次へ・名前は省略可）。写真をクリックでプレビュー、✕で取り込みから除外。",
  exclude_item: "取り込みから除外",
  include_item: "取り込みに戻す",
  excluded_warning: "{n} 件を取り込みから除外中",
  session_name_placeholder: "セッション名（例：奥津宮神社）",
  import_this_session: "このセッションを取り込む",
  stats: "写真 {photos} · 動画 {videos} · {size}",
  merge: "⤴ 結合",
  merge_title: "上のセッションに結合",
  preview: "プレビュー",
  back: "← 戻る",
  continue: "次へ →",
  import_to: "取り込み先",
  dest_placeholder: "写真アーカイブのフォルダ（「選択…」で指定）",
  browse: "選択…",
  start_import: "取り込み開始 →",
  checking: "確認中…",
  files_count: "{n} ファイル",
  importing: "取り込み中…",
  preparing: "準備中…",
  copying: "コピー中",
  verifying: "チェックサム検証中",
  cancel: "キャンセル",
  cancelling: "キャンセル中…",
  import_finished: "取り込み完了",
  import_cancelled: "取り込みをキャンセルしました",
  copied_summary: "✓ {n} ファイルをコピーして検証しました（{size}）",
  skipped_summary: "⏭ {n} ファイルはアーカイブに存在（スキップ）",
  failed_summary: "✕ {n} ファイルは失敗・未コピー — カードに残っています",
  failures: "失敗の詳細",
  delete_prompt:
    "検証済みの {n} ファイルを {name} から削除しますか？失敗・除外したファイルはそのまま残ります。",
  keep_files: "カードに残す",
  delete_files: "カードから {n} ファイルを削除",
  deleting: "削除中…",
  deleted_summary: "🧹 カードから {n} ファイルを削除しました。",
  delete_errors: "{n} 件を削除できませんでした。",
  eject: "⏏ カードを取り出す",
  ejected: "✓ 取り出しました — 安全に抜けます",
  import_another: "別のカードを取り込む",
};

export type MsgKey = keyof typeof en;

function systemLang(): Lang {
  return navigator.language?.toLowerCase().startsWith("ja") ? "ja" : "en";
}

export function currentLang(): Lang {
  const pref = wiz.settings.language;
  return pref === "system" ? systemLang() : pref;
}

/** Reactive: reads wiz.settings.language, so templates rerender on change. */
export function t(key: MsgKey, params?: Record<string, string | number>): string {
  let msg = (currentLang() === "ja" ? ja : en)[key];
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      msg = msg.replace(`{${k}}`, String(v));
    }
  }
  return msg;
}
