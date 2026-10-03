# Rust SkinID Studio

**Local, free, open-source** tool to create a custom icon and get a **unique
Steam Workshop SkinID** (new Workshop item via `ISteamUGC::CreateItem`).

Built for **custom items / plugins** (Oxide, etc.). It does **not** automatically
bind the icon to a vanilla Rust item. You get a SkinID, then use it in your configs.

---

## Download (end users)

Go to **[Releases](https://github.com/Mryococo91/rust-skinid-studio/releases)** and pick one:

| Package | Best for | Uninstall |
| --- | --- | --- |
| **Portable ZIP** (`*-portable-*-windows-x64.zip`) | USB / no admin / try quickly | Delete the folder — data is inside `app-data\` |
| **NSIS setup** (`*-setup.exe`) | Normal Windows install | Windows “Apps & features” → Uninstall (removes local data too) |
| **MSI** (`*.msi`) | Managed / enterprise installs | Same as above via Apps & features |

### First launch

1. Unzip (portable) or install (setup/MSI)
2. Open **Rust SkinID Studio**
3. Drop an image → **Connect / check Steam** → **Create a unique SkinID**
4. Steam must be running, logged into an account that **owns Rust**

Live Workshop publish may require `steam_api64.dll` next to the executable  
(Valve redistributable — see [docs/adr/0001-steamworks-redistribution.md](docs/adr/0001-steamworks-redistribution.md)).  
**Dry-run** works without Steam or the DLL.

---

## How it works

1. Import / crop an image (offline)
2. Sign in via the **local Steam client** (account that owns Rust)
3. **Create a unique SkinID** → new icon-only Workshop item
4. Copy the SkinID for your plugins / custom items

Updating an existing SkinID remains available as an advanced option.

---

## Develop

### Requirements

- Rust 1.80+
- **Bun** 1.3+
- Windows + MSVC Build Tools
- Steam client logged in (for live publish)

### Quick start

```bash
# core tests (dry-run, no Steam)
cargo test -p rss-core

# desktop
cd apps/desktop
bun install
bun run tauri:dev
```

### Build installers + portable locally

```bash
cd apps/desktop
bun install
bun run tauri:build

# from repo root — creates dist/release/*-portable-*-windows-x64.zip
pwsh ./scripts/package-portable.ps1 -Version 0.1.0
```

Artifacts:

- `target/release/bundle/nsis/*-setup.exe`
- `target/release/bundle/msi/*.msi`
- `dist/release/*-portable-*-windows-x64.zip`

### CLI

```bash
# dry-run
cargo run -p rss-spike --features steam -- spike \
  --image tests/fixtures/sample.png \
  --out tmp/spike-out \
  --mode dry-run

# live (Steam running)
cargo run -p rss-spike --features steam -- spike \
  --image tests/fixtures/sample.png \
  --out tmp/live-out \
  --mode live
```

---

## Release process

1. Bump version in `apps/desktop/package.json` and `apps/desktop/src-tauri/tauri.conf.json`
2. Commit, tag: `git tag v0.1.0 && git push origin v0.1.0`
3. GitHub Actions builds NSIS + MSI + portable ZIP and attaches them to the release

Details: [docs/runbooks/windows-packaging.md](docs/runbooks/windows-packaging.md)

---

## Privacy & safety

- No Steam password collection
- No remote backend / telemetry
- Uninstaller removes `%LOCALAPPDATA%\dev.rss.skinid-studio`
- Portable mode keeps all data in `./app-data` next to the exe

---

## License

MIT — see [LICENSE](LICENSE).  
Steamworks binaries: Valve license.
