; NSIS hooks for Rust SkinID Studio
; Always remove local app data on uninstall so nothing is left behind.

!macro NSIS_HOOK_PREINSTALL
!macroend

!macro NSIS_HOOK_POSTINSTALL
!macroend

!macro NSIS_HOOK_PREUNINSTALL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  SetShellVarContext current
  ; Current installed data dir (matches tauri identifier / ProjectStore::APP_DATA_FOLDER)
  RmDir /r "$LOCALAPPDATA\dev.rss.skinid-studio"
  RmDir /r "$APPDATA\dev.rss.skinid-studio"
  ; Legacy folder name from early builds
  RmDir /r "$LOCALAPPDATA\rust-supply-signal-studio"
  RmDir /r "$APPDATA\rust-supply-signal-studio"
!macroend
