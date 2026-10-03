# Runbook — Validate SkinID on `supply.signal`

## Goal

Confirm that a Workshop SkinID produced by this tool appears as the inventory
icon for `supply.signal` on a test server.

## Prerequisites

- Steam client logged in
- Account owns Rust (AppID 252490)
- App built with `--features steam` and `steam_api64.dll` present
- Workshop Legal Agreement accepted
- Test Oxide/uMod server (or local equivalent)

## Steps

1. Prepare a distinctive 512×512 PNG (bright color + unique mark).
2. Publish live:

```bash
cargo run -p rss-spike --features steam -- spike \
  --image tests/fixtures/sample.png \
  --out tmp/live-out \
  --mode live \
  --title "RSS Validation Signal"
```

3. Note `published_file_id` (SkinID) and open the Workshop URL.
4. Confirm the preview image matches the PNG.
5. On the test server, give a skinned item, e.g. (plugin-dependent):

```text
inventory.giveto <player> supply.signal 1 <SkinID>
```

or configure your custom-item / kits plugin with:

```json
{
  "shortname": "supply.signal",
  "skin": <SkinID>
}
```

6. In-game, verify the inventory icon matches the Workshop preview.

## If the icon does not apply

1. Retry with the same pack (ensure tags `Version3`, `Skin`).
2. Confirm `manifest.txt` has empty Textures and Version 3.
3. Experimentally change `ItemType` (start with `Grenade`) and republish as a
   **new** item.
4. Confirm the player/client can download Workshop previews (subscription /
   server-side skin application path).
5. Record the Facepunch / Rust build date — client changes can break icon-only.

## Dry-run checklist (no Steam)

```bash
cargo test -p rss-core
cargo run -p rss-spike -- spike --image tests/fixtures/sample.png --out tmp/spike-out --mode dry-run
```

Expect synthetic SkinID `9000000001+` and packed files under `tmp/spike-out`.
