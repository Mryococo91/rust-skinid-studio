# ADR 0001 — Steamworks SDK redistribution

## Status

Accepted

## Context

Live Workshop publishing requires Valve's Steamworks redistributable
(`steam_api64.dll` on Windows) and the `ISteamUGC` API.

Valve's Steamworks SDK Access Agreement allows redistributing only the contents
of `redistributable_bin` with an application. The full SDK must not be published
in source repositories or npm packages.

## Decision

1. Do **not** commit `steam_api64.dll` or the Steamworks SDK to git.
2. Default builds use **dry-run** mock publishing (no DLL required).
3. Live publishing is gated behind Cargo feature `steam`.
4. Document how end users / packagers obtain the redistributable legally.
5. Prefer loading the DLL next to the executable at runtime.

## Consequences

- CI can test the full offline pipeline without Steam.
- Live publish requires Steam client + Rust ownership + redistributable DLL.
- Packagers must follow Valve's license when shipping live builds.
