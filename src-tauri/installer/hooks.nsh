; NSIS installer hooks for LanBlaze.
; Adds a "Send via LanBlaze" entry to the Windows Explorer right-click
; menu for both files (HKCU\Software\Classes\*\shell\...) and folders
; (HKCU\Software\Classes\Directory\shell\...). Per-user only — no UAC.
; Tauri's NSIS template calls these macros at install / uninstall time.

!macro NSIS_HOOK_POSTINSTALL
  ; --- Files --------------------------------------------------------
  WriteRegStr HKCU "Software\Classes\*\shell\LanBlaze" "" "Send via LanBlaze"
  WriteRegStr HKCU "Software\Classes\*\shell\LanBlaze" "Icon" '"$INSTDIR\${MAINBINARYNAME}.exe"'
  WriteRegStr HKCU "Software\Classes\*\shell\LanBlaze\command" "" '"$INSTDIR\${MAINBINARYNAME}.exe" "%1"'

  ; --- Folders ------------------------------------------------------
  WriteRegStr HKCU "Software\Classes\Directory\shell\LanBlaze" "" "Send via LanBlaze"
  WriteRegStr HKCU "Software\Classes\Directory\shell\LanBlaze" "Icon" '"$INSTDIR\${MAINBINARYNAME}.exe"'
  WriteRegStr HKCU "Software\Classes\Directory\shell\LanBlaze\command" "" '"$INSTDIR\${MAINBINARYNAME}.exe" "%1"'

  ; --- Background of a folder (right-click empty space) -------------
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\LanBlaze" "" "Send via LanBlaze"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\LanBlaze" "Icon" '"$INSTDIR\${MAINBINARYNAME}.exe"'
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\LanBlaze\command" "" '"$INSTDIR\${MAINBINARYNAME}.exe" "%V"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCU "Software\Classes\*\shell\LanBlaze"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\LanBlaze"
  DeleteRegKey HKCU "Software\Classes\Directory\Background\shell\LanBlaze"
!macroend
