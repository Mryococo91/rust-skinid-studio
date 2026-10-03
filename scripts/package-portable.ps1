# Build a portable ZIP: unzip anywhere, run, delete folder = no traces.
# Prerequisite: `bun run tauri:build` (or CI release build) already completed.

param(
    [string]$Version = "0.1.0",
    [string]$OutDir = "dist/release"
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $Root

$ExeName = "rust-supply-signal-studio.exe"
$Candidates = @(
    (Join-Path $Root "target\release\$ExeName"),
    (Join-Path $Root "apps\desktop\src-tauri\target\release\$ExeName")
)

$Exe = $Candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $Exe) {
    Write-Error "Executable not found. Run: cd apps/desktop; bun run tauri:build"
}

$StageName = "RustSkinIDStudio-portable-$Version-windows-x64"
$Stage = Join-Path $Root (Join-Path $OutDir $StageName)
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force -Path $Stage | Out-Null

Copy-Item $Exe (Join-Path $Stage "Rust SkinID Studio.exe")
Copy-Item (Join-Path $Root "LICENSE") (Join-Path $Stage "LICENSE")
Copy-Item (Join-Path $Root "apps\desktop\src-tauri\resources\README-steam-dll.txt") `
    (Join-Path $Stage "README-steam-dll.txt")

# Marker: app stores data in ./app-data next to the exe (see ProjectStore::default_data_dir)
New-Item -ItemType File -Path (Join-Path $Stage "PORTABLE") -Force | Out-Null
Set-Content -Path (Join-Path $Stage "PORTABLE") -Value "Rust SkinID Studio portable mode`nDelete this folder to remove the app completely."

@"
# Rust SkinID Studio — Portable

1. Keep this whole folder together.
2. Double-click **Rust SkinID Studio.exe**
3. For live Steam publish, place ``steam_api64.dll`` next to the exe
   (see README-steam-dll.txt). Dry-run works without it.
4. Local data lives in ``app-data\`` (created on first launch).
5. To uninstall: close the app and delete this folder — nothing else remains.

Steam must be running (account that owns Rust) for live SkinID creation.
"@ | Set-Content -Path (Join-Path $Stage "README.txt") -Encoding UTF8

New-Item -ItemType Directory -Force -Path (Join-Path $Root $OutDir) | Out-Null
$Zip = Join-Path $Root (Join-Path $OutDir "$StageName.zip")
if (Test-Path $Zip) { Remove-Item -Force $Zip }
Compress-Archive -Path $Stage -DestinationPath $Zip -Force

Write-Host "Portable package ready:"
Write-Host "  $Zip"
Write-Host "  Folder: $Stage"
