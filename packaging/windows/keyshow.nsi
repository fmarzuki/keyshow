; Keyshow installer for Windows (NSIS 3).
; Per-user install: no administrator rights needed.
;
;   makensis -DVERSION=0.1.0 -DBINARY=path/to/keyshow.exe -DOUTFILE=path/to/setup.exe packaging/windows/keyshow.nsi
;
; Paths below are relative to this file's directory.

Unicode True

!ifndef VERSION
  !define VERSION "0.1.0"
!endif
!ifndef BINARY
  !define BINARY "../../target/x86_64-pc-windows-gnu/release/keyshow.exe"
!endif
!ifndef OUTFILE
  !define OUTFILE "../../dist/keyshow-${VERSION}-windows-x86_64-setup.exe"
!endif

!define APP "Keyshow"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Keyshow"
!define RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"

!include "MUI2.nsh"
!include "FileFunc.nsh"

Name "${APP}"
OutFile "${OUTFILE}"
RequestExecutionLevel user
InstallDir "$LOCALAPPDATA\Programs\Keyshow"
InstallDirRegKey HKCU "Software\Keyshow" "InstallDir"
SetCompressor /SOLID lzma
BrandingText "${APP} ${VERSION}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=1033 "ProductName" "${APP}"
VIAddVersionKey /LANG=1033 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "FileDescription" "${APP} installer"
VIAddVersionKey /LANG=1033 "LegalCopyright" "Farkhan Marzuki"

!define MUI_ICON "keyshow.ico"
!define MUI_UNICON "keyshow.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\keyshow.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Start Keyshow now"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Indonesian"

; Ask a running Keyshow to quit so its files can be replaced or removed.
!macro StopKeyshow
  IfFileExists "$INSTDIR\keyshow.exe" 0 +3
    ExecWait '"$INSTDIR\keyshow.exe" --quit'
    Sleep 1500
!macroend

Section "Keyshow (required)" SecMain
  SectionIn RO
  !insertmacro StopKeyshow
  SetOutPath "$INSTDIR"
  File "/oname=keyshow.exe" "${BINARY}"
  File "/oname=README.md" "../../README.md"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortcut "$SMPROGRAMS\Keyshow.lnk" "$INSTDIR\keyshow.exe" "" "$INSTDIR\keyshow.exe" 0 SW_SHOWNORMAL "" "Show your keystrokes on screen"

  WriteRegStr HKCU "Software\Keyshow" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "Farkhan Marzuki"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\keyshow.exe"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" "$0"
SectionEnd

Section "Desktop shortcut" SecDesktop
  CreateShortcut "$DESKTOP\Keyshow.lnk" "$INSTDIR\keyshow.exe"
SectionEnd

Section /o "Start Keyshow when I sign in" SecAutostart
  WriteRegStr HKCU "${RUN_KEY}" "Keyshow" '"$INSTDIR\keyshow.exe"'
SectionEnd

LangString DESC_Main ${LANG_ENGLISH} "The Keyshow app and a Start menu shortcut."
LangString DESC_Main ${LANG_INDONESIAN} "Aplikasi Keyshow dan shortcut di Start menu."
LangString DESC_Desktop ${LANG_ENGLISH} "Add a Keyshow icon to the desktop."
LangString DESC_Desktop ${LANG_INDONESIAN} "Tambahkan ikon Keyshow di desktop."
LangString DESC_Autostart ${LANG_ENGLISH} "Run Keyshow automatically when you sign in to Windows."
LangString DESC_Autostart ${LANG_INDONESIAN} "Jalankan Keyshow otomatis saat masuk ke Windows."
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecMain} $(DESC_Main)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} $(DESC_Desktop)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecAutostart} $(DESC_Autostart)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Section "Uninstall"
  !insertmacro StopKeyshow
  Delete "$INSTDIR\keyshow.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Keyshow.lnk"
  Delete "$DESKTOP\Keyshow.lnk"
  DeleteRegValue HKCU "${RUN_KEY}" "Keyshow"
  DeleteRegKey HKCU "${UNINST_KEY}"
  DeleteRegKey HKCU "Software\Keyshow"
  ; Settings in %APPDATA%\keyshow are kept on purpose.
SectionEnd
