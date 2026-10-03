# ADR 0002 — Icon-only Supply Signal skins

## Status

Accepted

## Context

Facepunch's official Workshop skin pipeline targets skinnable items with
replaceable textures. Supply Signal is not an officially skinnable 3D item.

Server plugin ecosystems use Workshop items as **inventory icons** via SkinID.

## Decision

Ship an **icon-only** Workshop pack:

- `manifest.txt` Version 3
- `ItemType`: `Grenade` (community convention for signal-like items)
- empty `Textures` map
- Workshop preview PNG = in-game inventory icon
- tags: `Version3`, `Skin`

Clearly document that this does **not** change the 3D/world model.

## Consequences

- Product matches Custom Icon Creator / SkyPlugins use cases.
- Behavior depends on undocumented Facepunch client conventions and may break.
- Phase 0 spike + Oxide runbook required for empirical validation.
