# Runbook — Windows packaging

## Goals

Ship two end-user friendly options on every GitHub Release:

1. **Portable ZIP** — unzip, run, delete folder = clean
2. **NSIS installer** — install + uninstall with **no leftover app data**
3. **MSI** — alternative installer for managed environments

## Dev / release build

```bash
cd apps/desktop
bun install
bun run tauri:build
```

From repo root, package the portable ZIP:

```powershell
pwsh ./scripts/package-portable.ps1 -Version 0.1.0
```

Outputs:

| Artifact | Path |
| --- | --- |
| NSIS setup | `target/release/bundle/nsis/*-setup.exe` |
| MSI | `target/release/bundle/msi/*.msi` |
| Portable | `dist/release/RustSkinIDStudio-portable-<ver>-windows-x64.zip` |

## Portable mode

The portable ZIP includes a `PORTABLE` marker file next to the exe.

- App data → `./app-data` (created on first launch)
- Delete the whole folder → nothing left on the machine
- Override paths with `RSS_DATA_DIR` or force portable with `RSS_PORTABLE=1`

## Installer uninstall (no leftovers)

NSIS hooks (`apps/desktop/src-tauri/nsis/hooks.nsh`) always remove:

- `%LOCALAPPDATA%\dev.rss.skinid-studio`
- `%APPDATA%\dev.rss.skinid-studio`
- legacy `%LOCALAPPDATA%\rust-supply-signal-studio`

Install mode: **current user** (no admin required by default).

## Live Steam build notes

1. Obtain Steamworks SDK redistributable (`steam_api64.dll`).
2. Place DLL next to the built executable for live publish.
3. Do **not** commit Valve binaries (see ADR 0001).
4. Include `resources/README-steam-dll.txt` and LICENSE (already bundled).

CI release builds with `--features steam`. The DLL is still required at
**runtime** for live Workshop calls; dry-run works without it.

## Smoke test after install / unzip

1. Launch without Steam → dry-run publish works.
2. With Steam + DLL → Connect / check Steam reports ready when Rust is owned.
3. Uninstall (or delete portable folder) → verify AppData folders are gone.
4. Settings → Wipe local data clears projects/history without uninstalling.

## Tag a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions workflow `.github/workflows/release.yml` builds and uploads all artifacts.
