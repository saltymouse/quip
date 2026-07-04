<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="128" alt="quip app icon: two quotation marks as the sails of a small boat">
</p>

<p align="center">English | <a href="README.ja.md">日本語</a></p>

# quip

Quick import photo: a small macOS app that pulls photos and videos off a
camera SD card, groups them into dated sessions, and files them into a photo
archive with verified copies. Built for one job: card in, named folders out, card wiped.
It reads the standard DCIM layout every camera writes (plus Sony's video
sidecars when present), or any plain folder of media. Only tested on macOS
Tahoe so far.

![The sessions screen: a day's shots grouped and ready to name](docs/sessions-screen.png)

## How an import works

1. quip watches `/Volumes` and lists any card with a `DCIM` folder. One card
   plugged in → scanned automatically. Nothing mounted → pick a folder by hand.
2. Shots are grouped into sessions by capture date (EXIF, Sony XML sidecars,
   or file timestamps). Shots before 4 AM count as the previous day, so a
   shoot that runs past midnight stays together. Name each session (optional,
   Enter accepts), merge groups, or ✕ a thumbnail to leave it behind.
3. Pick a destination (remembered). The folder layout is a pattern built from
   `{year}` `{month}` `{day}` `{date}` `{name}`, with a live preview;
   the default is `{year}/{date} {name}`.
4. Every file is copied with an in-flight blake3 checksum, then read back from
   the destination and compared. Same name + size elsewhere → skipped as a
   duplicate; name clash → kept as `name (2).ext`.
5. The new folders open in Finder, and you can delete the verified files from
   the card (including macOS `._*` junk and Sony sidecars), then eject.

Settings persist across launches. The UI is bilingual (English / 日本語),
following the system language with an override in settings.

## Project layout

A Tauri app is two programs in one repo: a Rust binary in `src-tauri/` that
owns the window and does the privileged work, and a Svelte 5 + TypeScript
frontend in `src/` rendering inside it. The frontend calls Rust over
`invoke()`; Rust pushes progress back as events.

```
quip/
├── src/                        frontend (SvelteKit SPA, bun)
│   ├── routes/+page.svelte     wizard shell and step routing
│   └── lib/
│       ├── api.ts              typed wrappers around every invoke() command
│       ├── wizard.svelte.ts    shared state ($state runes) + saved settings
│       ├── i18n.svelte.ts      en/ja strings
│       └── components/         one .svelte file per wizard step
└── src-tauri/                  backend (Rust)
    ├── tauri.conf.json         window, bundle id, build commands
    ├── capabilities/           what the webview may call
    └── src/
        ├── volumes.rs          /Volumes watcher, card detection, eject
        ├── scan.rs             capture dates, session grouping
        ├── thumbs.rs           thumbnail cache (EXIF-embedded fast path)
        └── import.rs           copy, blake3 verify, delete, reveal
```

## Development

Needs bun and a Rust toolchain (`rustup`).

```sh
bun install          # once
bun run tauri dev    # run with hot reload

cd src-tauri && cargo test    # Rust unit + integration tests
bun run check                 # svelte-check
```

## Building

`bun run tauri build` produces `quip.app` and a `.dmg` under
`src-tauri/target/release/bundle/`. Drag the `.app` into `/Applications`.
