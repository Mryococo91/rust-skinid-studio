Steamworks redistributable (steam_api64.dll)
===========================================

This application can publish to the Steam Workshop using Valve's Steamworks
redistributable binary (steam_api64.dll) when built with the `steam` feature.

Due to Valve's Steamworks SDK Access Agreement, steam_api64.dll is NOT
committed to this repository.

To enable live publishing:
1. Register for free at https://partner.steamgames.com/
2. Download the Steamworks SDK
3. Copy redistributable_bin/win64/steam_api64.dll next to the app executable
   (or into apps/desktop/src-tauri/ when developing)
4. Build with: cargo build -p rust-supply-signal-studio --features steam
5. Run Steam client logged into an account that owns Rust (AppID 252490)

Dry-run publishing works without the DLL.
