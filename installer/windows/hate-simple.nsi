; Hate Programming Language - NSIS Installer Script (Simplified)
; Build with: makensis hate-simple.nsi
;
; Run from installer/windows directory with build/ folder containing:
;   - hate.exe
;   - README.md
;   - docs/
;   - examples/

!include "MUI2.nsh"

;--------------------------------
; Configuration

!define PRODUCT_NAME "Hate"
!define PRODUCT_VERSION "0.1.0"
!define PRODUCT_PUBLISHER "Hate Team"
!define PRODUCT_WEB_SITE "https://github.com/tafolabi009/hate"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "hate-${PRODUCT_VERSION}-windows-x86_64-setup.exe"
InstallDir "$PROGRAMFILES64\Hate"
RequestExecutionLevel admin
ShowInstDetails show

;--------------------------------
; Modern UI

!define MUI_ABORTWARNING

; Welcome page
!define MUI_WELCOMEPAGE_TITLE "Welcome to Hate ${PRODUCT_VERSION} Setup"
!define MUI_WELCOMEPAGE_TEXT "Hate is a blazingly fast programming language.$\r$\n$\r$\nFeatures:$\r$\n  • Register-based bytecode VM$\r$\n  • NaN-boxing for efficient values$\r$\n  • Modern syntax with pattern matching$\r$\n$\r$\nClick Next to continue."

; Finish page
!define MUI_FINISHPAGE_RUN "$INSTDIR\hate.exe"
!define MUI_FINISHPAGE_RUN_PARAMETERS "repl"
!define MUI_FINISHPAGE_RUN_TEXT "Start Hate REPL"
!define MUI_FINISHPAGE_LINK "Visit Hate website"
!define MUI_FINISHPAGE_LINK_LOCATION "${PRODUCT_WEB_SITE}"

;--------------------------------
; Pages

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

;--------------------------------
; Languages

!insertmacro MUI_LANGUAGE "English"

;--------------------------------
; Installer Section

Section "Install"
    SetOutPath "$INSTDIR"
    
    ; Core files
    File "build\hate.exe"
    File "build\README.md"
    
    ; Documentation
    SetOutPath "$INSTDIR\docs"
    File /r "build\docs\*.*"
    
    ; Examples
    SetOutPath "$INSTDIR\examples"
    File /r "build\examples\*.*"
    
    ; Add to PATH
    EnVar::SetHKLM
    EnVar::AddValue "PATH" "$INSTDIR"
    
    ; Create uninstaller
    WriteUninstaller "$INSTDIR\uninst.exe"
    
    ; Registry for Add/Remove Programs
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "UninstallString" "$INSTDIR\uninst.exe"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"
    
    ; Start Menu
    CreateDirectory "$SMPROGRAMS\Hate"
    CreateShortCut "$SMPROGRAMS\Hate\Hate REPL.lnk" "$INSTDIR\hate.exe" "repl"
    CreateShortCut "$SMPROGRAMS\Hate\Uninstall.lnk" "$INSTDIR\uninst.exe"
    
    ; Desktop shortcut
    CreateShortCut "$DESKTOP\Hate REPL.lnk" "$INSTDIR\hate.exe" "repl"
    
    ; File association
    WriteRegStr HKCR ".hate" "" "HateScript"
    WriteRegStr HKCR "HateScript" "" "Hate Script File"
    WriteRegStr HKCR "HateScript\shell\open\command" "" '"$INSTDIR\hate.exe" "%1"'
SectionEnd

;--------------------------------
; Uninstaller Section

Section "Uninstall"
    ; Remove from PATH
    EnVar::SetHKLM
    EnVar::DeleteValue "PATH" "$INSTDIR"
    
    ; Remove files
    Delete "$INSTDIR\hate.exe"
    Delete "$INSTDIR\README.md"
    Delete "$INSTDIR\uninst.exe"
    RMDir /r "$INSTDIR\docs"
    RMDir /r "$INSTDIR\examples"
    RMDir "$INSTDIR"
    
    ; Remove shortcuts
    Delete "$DESKTOP\Hate REPL.lnk"
    RMDir /r "$SMPROGRAMS\Hate"
    
    ; Remove registry
    DeleteRegKey HKLM "${PRODUCT_UNINST_KEY}"
    DeleteRegKey HKCR ".hate"
    DeleteRegKey HKCR "HateScript"
SectionEnd
