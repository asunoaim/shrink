; "Open in shrink" in Explorer's right-click menu for video files (per user, no admin).
; Windows 11 shows it under "Show more options".

!macro SHRINK_MENU_ADD EXT
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\shrink" "" "Open in shrink"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\shrink" "Icon" "$INSTDIR\shrink.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\shrink\command" "" '"$INSTDIR\shrink.exe" "%1"'
!macroend

!macro SHRINK_MENU_REMOVE EXT
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\${EXT}\shell\shrink"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro SHRINK_MENU_ADD ".mp4"
  !insertmacro SHRINK_MENU_ADD ".mkv"
  !insertmacro SHRINK_MENU_ADD ".mov"
  !insertmacro SHRINK_MENU_ADD ".m4v"
  !insertmacro SHRINK_MENU_ADD ".webm"
  !insertmacro SHRINK_MENU_ADD ".avi"
  !insertmacro SHRINK_MENU_ADD ".ts"
  !insertmacro SHRINK_MENU_ADD ".flv"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro SHRINK_MENU_REMOVE ".mp4"
  !insertmacro SHRINK_MENU_REMOVE ".mkv"
  !insertmacro SHRINK_MENU_REMOVE ".mov"
  !insertmacro SHRINK_MENU_REMOVE ".m4v"
  !insertmacro SHRINK_MENU_REMOVE ".webm"
  !insertmacro SHRINK_MENU_REMOVE ".avi"
  !insertmacro SHRINK_MENU_REMOVE ".ts"
  !insertmacro SHRINK_MENU_REMOVE ".flv"
!macroend
