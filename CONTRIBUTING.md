# Contributing

Thanks for helping improve Rust SkinID Studio.

## Setup

1. Install Rust (stable), Bun 1.3+, and MSVC Build Tools (Windows).
2. `cargo test -p rss-core`
3. `cd apps/desktop && bun install && bun run tauri:dev`

## Rules

- Keep business logic in `rss-core`, not in React components.
- Do not commit `steam_api64.dll` or the Steamworks SDK.
- Prefer small, tested changes.
- Match existing naming: product = **Rust SkinID Studio**, crates keep `rss-*`.

## Checks before PR

```bash
cargo test -p rss-core
cd apps/desktop && bun run build
cargo check -p rust-supply-signal-studio
```

## Releases

Maintainers bump version, tag `vX.Y.Z`, and let GitHub Actions publish installers + portable ZIP.
