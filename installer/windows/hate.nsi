; Hate Programming Language - NSIS Installer Script
; https://github.com/tafolabi009/hate
;
; Build with: makensis hate.nsi
; Requires NSIS 3.x

;--------------------------------
; Includes

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "EnvVarUpdate.nsh"
!include "x64.nsh"

;--------------------------------
; General Configuration

!define PRODUCT_NAME "Hate"
!define PRODUCT_VERSION "0.1.0"
!define PRODUCT_PUBLISHER "Hate Team"
!define PRODUCT_WEB_SITE "https://github.com/tafolabi009/hate"
!define PRODUCT_DIR_REGKEY "Software\Microsoft\Windows\CurrentVersion\App Paths\hate.exe"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
!define PRODUCT_UNINST_ROOT_KEY "HKLM"

; Installer attributes
Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "hate-${PRODUCT_VERSION}-windows-x86_64-setup.exe"
InstallDir "$PROGRAMFILES64\Hate"
InstallDirRegKey HKLM "${PRODUCT_DIR_REGKEY}" ""
RequestExecutionLevel admin
ShowInstDetails show
ShowUnInstDetails show

; Compression
SetCompressor /SOLID lzma

;--------------------------------
; Modern UI Configuration

!define MUI_ABORTWARNING
!define MUI_ICON "hate.ico"
!define MUI_UNICON "hate.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "header.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "welcome.bmp"

; Welcome page
!define MUI_WELCOMEPAGE_TITLE "Welcome to Hate ${PRODUCT_VERSION} Setup"
!define MUI_WELCOMEPAGE_TEXT "This wizard will guide you through the installation of Hate, a blazingly fast programming language.$\r$\n$\r$\nHate features:$\r$\n  • Register-based bytecode VM$\r$\n  • NaN-boxing for efficient values$\r$\n  • Generational garbage collector$\r$\n  • Modern syntax with pattern matching$\r$\n$\r$\nClick Next to continue."

; Finish page
!define MUI_FINISHPAGE_RUN "$INSTDIR\hate.exe"
!define MUI_FINISHPAGE_RUN_PARAMETERS "repl"
!define MUI_FINISHPAGE_RUN_TEXT "Start Hate REPL"
!define MUI_FINISHPAGE_LINK "Visit Hate website"
!define MUI_FINISHPAGE_LINK_LOCATION "${PRODUCT_WEB_SITE}"
!define MUI_FINISHPAGE_SHOWREADME "$INSTDIR\README.md"
!define MUI_FINISHPAGE_SHOWREADME_TEXT "View README"

;--------------------------------
; Installer Pages

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\..\LICENSE"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

;--------------------------------
; Uninstaller Pages

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

;--------------------------------
; Languages

!insertmacro MUI_LANGUAGE "English"

;--------------------------------
; Version Information

VIProductVersion "${PRODUCT_VERSION}.0"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductName" "${PRODUCT_NAME}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductVersion" "${PRODUCT_VERSION}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "CompanyName" "${PRODUCT_PUBLISHER}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "LegalCopyright" "Copyright (c) 2024 ${PRODUCT_PUBLISHER}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileDescription" "Hate Programming Language Installer"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileVersion" "${PRODUCT_VERSION}"

;--------------------------------
; Installer Sections

Section "Hate Runtime (required)" SEC_RUNTIME
    SectionIn RO
    
    SetOutPath "$INSTDIR"
    SetOverwrite on
    
    ; Core files
    File "..\..\target\x86_64-pc-windows-gnu\release\hate.exe"
    File "..\..\README.md"
    File "..\..\LICENSE"
    
    ; Create docs directory
    SetOutPath "$INSTDIR\docs"
    File /r "..\..\docs\*.*"
    
    ; Create examples directory
    SetOutPath "$INSTDIR\examples"
    File /r "..\..\examples\*.*"
    
    ; Write registry keys
    WriteRegStr HKLM "${PRODUCT_DIR_REGKEY}" "" "$INSTDIR\hate.exe"
    WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayName" "$(^Name)"
    WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "UninstallString" "$INSTDIR\uninst.exe"
    WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayIcon" "$INSTDIR\hate.exe"
    WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
    WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"
    WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
    
    ; Get installed size
    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "EstimatedSize" "$0"
    
    ; Create uninstaller
    WriteUninstaller "$INSTDIR\uninst.exe"
SectionEnd

Section "Add to PATH" SEC_PATH
    ; Add to system PATH
    ${EnvVarUpdate} $0 "PATH" "A" "HKLM" "$INSTDIR"
    
    ; Set HATE_HOME environment variable
    WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "HATE_HOME" "$INSTDIR"
    
    ; Notify about environment change
    SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
SectionEnd

Section "Desktop Shortcut" SEC_DESKTOP
    CreateShortCut "$DESKTOP\Hate REPL.lnk" "$INSTDIR\hate.exe" "repl" "$INSTDIR\hate.exe" 0
SectionEnd

Section "Start Menu Shortcuts" SEC_STARTMENU
    CreateDirectory "$SMPROGRAMS\Hate"
    CreateShortCut "$SMPROGRAMS\Hate\Hate REPL.lnk" "$INSTDIR\hate.exe" "repl" "$INSTDIR\hate.exe" 0
    CreateShortCut "$SMPROGRAMS\Hate\Hate Documentation.lnk" "$INSTDIR\docs\tutorial.md"
    CreateShortCut "$SMPROGRAMS\Hate\Uninstall Hate.lnk" "$INSTDIR\uninst.exe"
SectionEnd

Section "File Association (.hate)" SEC_ASSOC
    ; Register .hate extension
    WriteRegStr HKCR ".hate" "" "HateScript"
    WriteRegStr HKCR "HateScript" "" "Hate Script"
    WriteRegStr HKCR "HateScript\DefaultIcon" "" "$INSTDIR\hate.exe,0"
    WriteRegStr HKCR "HateScript\shell\open\command" "" '"$INSTDIR\hate.exe" "%1"'
    WriteRegStr HKCR "HateScript\shell\edit" "" "Edit with Notepad"
    WriteRegStr HKCR "HateScript\shell\edit\command" "" 'notepad.exe "%1"'
    
    ; Register .hatec extension (compiled bytecode)
    WriteRegStr HKCR ".hatec" "" "HateCompiled"
    WriteRegStr HKCR "HateCompiled" "" "Hate Compiled Bytecode"
    WriteRegStr HKCR "HateCompiled\DefaultIcon" "" "$INSTDIR\hate.exe,0"
    WriteRegStr HKCR "HateCompiled\shell\open\command" "" '"$INSTDIR\hate.exe" run "%1"'
    
    ; Refresh shell
    System::Call 'Shell32::SHChangeNotify(i 0x8000000, i 0, i 0, i 0)'
SectionEnd

;--------------------------------
; Section Descriptions

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${SEC_RUNTIME} "Core Hate runtime, documentation, and examples. (Required)"
    !insertmacro MUI_DESCRIPTION_TEXT ${SEC_PATH} "Add Hate to the system PATH for command-line access."
    !insertmacro MUI_DESCRIPTION_TEXT ${SEC_DESKTOP} "Create a desktop shortcut to Hate REPL."
    !insertmacro MUI_DESCRIPTION_TEXT ${SEC_STARTMENU} "Create Start Menu shortcuts."
    !insertmacro MUI_DESCRIPTION_TEXT ${SEC_ASSOC} "Associate .hate files with Hate interpreter."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

;--------------------------------
; Installer Functions

Function .onInit
    ; Check for 64-bit Windows
    ${IfNot} ${RunningX64}
        MessageBox MB_OK|MB_ICONSTOP "This installer requires 64-bit Windows."
        Abort
    ${EndIf}
    
    ; Check for admin rights
    UserInfo::GetAccountType
    Pop $0
    ${If} $0 != "admin"
        MessageBox MB_OK|MB_ICONSTOP "Administrator privileges required for installation."
        Abort
    ${EndIf}
FunctionEnd

;--------------------------------
; Uninstaller Section

Section "Uninstall"
    ; Remove from PATH
    ${un.EnvVarUpdate} $0 "PATH" "R" "HKLM" "$INSTDIR"
    
    ; Remove HATE_HOME
    DeleteRegValue HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "HATE_HOME"
    
    ; Remove file associations
    DeleteRegKey HKCR ".hate"
    DeleteRegKey HKCR ".hatec"
    DeleteRegKey HKCR "HateScript"
    DeleteRegKey HKCR "HateCompiled"
    
    ; Remove shortcuts
    Delete "$DESKTOP\Hate REPL.lnk"
    RMDir /r "$SMPROGRAMS\Hate"
    
    ; Remove files
    Delete "$INSTDIR\hate.exe"
    Delete "$INSTDIR\README.md"
    Delete "$INSTDIR\LICENSE"
    Delete "$INSTDIR\uninst.exe"
    RMDir /r "$INSTDIR\docs"
    RMDir /r "$INSTDIR\examples"
    RMDir "$INSTDIR"
    
    ; Remove registry keys
    DeleteRegKey ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}"
    DeleteRegKey HKLM "${PRODUCT_DIR_REGKEY}"
    
    ; Refresh shell
    System::Call 'Shell32::SHChangeNotify(i 0x8000000, i 0, i 0, i 0)'
    
    ; Notify about environment change
    SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
SectionEnd
