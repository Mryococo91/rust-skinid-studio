# Security

## Authentication

- Steam password is **never** requested or stored.
- Live mode uses the local Steam client session via Steamworks `SteamAPI_Init`.
- Dry-run mode never contacts Steam.

## Data

- Local SQLite + project folders only.
- No telemetry, analytics, or remote backends.
- No third-party upload of images.

## Steamworks redistribution

- Only Valve `redistributable_bin` may ship with releases.
- Full SDK must not be committed.
- See ADR 0001.

## Threat notes

- A compromised local app could publish Workshop items as the logged-in Steam user.
- Mitigate by defaulting to dry-run and clear live-mode UX.
