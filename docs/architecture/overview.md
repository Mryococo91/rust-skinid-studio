# Architecture Overview — Rust SkinID Studio

## Purpose

Local desktop tool to prepare a square PNG icon and publish an **icon-only**
Rust Steam Workshop item, producing a SkinID (`PublishedFileId`) usable with
custom items / Oxide plugins.

## Stack

- **UI:** React + TypeScript + Vite (Tauri webview)
- **Shell:** Tauri 2
- **Core:** `rss-core` (Rust) — image, asset, steam, workshop, project
- **CLI spike:** `rss-spike` (Phase 0 feasibility)
- **Storage:** SQLite (local projects + publish history)
  - Installed: `%LOCALAPPDATA%\dev.rss.skinid-studio`
  - Portable (`PORTABLE` marker): `./app-data` next to the exe

## Layers

```text
React UI
  → Tauri commands
    → rss-core
      → filesystem / SQLite
      → Steamworks ISteamUGC (optional feature `steam`)
```

## Offline vs online

| Capability | Offline | Needs Steam |
|------------|---------|-------------|
| Import / crop / resize / validate | Yes | No |
| Pack `manifest.txt` + preview | Yes | No |
| Dry-run publish | Yes | No |
| Live Workshop publish/update | No | Yes |

## Security

- No Steam password collection
- No remote backend
- No telemetry
- Steam auth = local Steam client session only
