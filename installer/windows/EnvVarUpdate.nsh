/**
 *  EnvVarUpdate.nsh
 *    : Environmental Variables: append, prepend, and remove entries
 *
 *     WARNING: If you use StrFunc.nsh header then include it before this file
 *              with all required definitions. This is to avoid conflicts
 *
 *  Usage:
 *    ${EnvVarUpdate} $0 "MYVAR" "A|P|R" "HKLM|HKCU" "new_value"
 *
 *    A = Append
 *    P = Prepend
 *    R = Remove
 *
 *  Example:
 *    ${EnvVarUpdate} $0 "PATH" "A" "HKLM" "C:\MyApp"
 *    ${EnvVarUpdate} $0 "PATH" "P" "HKLM" "C:\MyApp"
 *    ${EnvVarUpdate} $0 "PATH" "R" "HKLM" "C:\MyApp"
 *
 */
 
!ifndef ENVVARUPDATE_NSH
!define ENVVARUPDATE_NSH
 
!include "LogicLib.nsh"
!include "WinMessages.nsh"
 
!ifndef StrFunc_nsh_Defined
    !include "StrFunc.nsh"
    ${StrTok}
    ${StrStr}
    ${StrRep}
!endif
 
!ifndef LVM_GETITEMCOUNT
    !define LVM_GETITEMCOUNT 0x1004
!endif
!ifndef LVM_GETITEMTEXT
    !define LVM_GETITEMTEXT 0x102D
!endif
 
!define hEnvVarUpdate '!insertmacro hEnvVarUpdate'
 
!macro hEnvVarUpdate un
Function ${un}hEnvVarUpdate
    Push $0
    Exch 4
    Exch $1
    Exch 3
    Exch $2
    Exch 2
    Exch $3
    Exch
    Exch $4
    Push $5
    Push $6
    Push $7
    Push $8
    Push $9
    Push $R0
 
    ${If} $2 == "HKLM"
        StrCpy $5 "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"
    ${ElseIf} $2 == "HKCU"
        StrCpy $5 "Environment"
    ${Else}
        Goto hEnvVarUpdate_done
    ${EndIf}
 
    ${If} $2 == "HKLM"
        ReadRegStr $6 HKLM $5 $1
    ${Else}
        ReadRegStr $6 HKCU $5 $1
    ${EndIf}
 
    ; Check if value already exists
    ${StrStr} $7 $6 $4
    ${If} $7 != ""
        ; Value exists, check if it's an exact match
        ${If} $3 == "R"
            ; Remove value
            ${StrRep} $6 $6 ";$4" ""
            ${StrRep} $6 $6 "$4;" ""
            ${StrRep} $6 $6 "$4" ""
        ${EndIf}
    ${Else}
        ; Value doesn't exist
        ${If} $3 == "A"
            ; Append
            ${If} $6 != ""
                StrCpy $6 "$6;$4"
            ${Else}
                StrCpy $6 "$4"
            ${EndIf}
        ${ElseIf} $3 == "P"
            ; Prepend
            ${If} $6 != ""
                StrCpy $6 "$4;$6"
            ${Else}
                StrCpy $6 "$4"
            ${EndIf}
        ${EndIf}
    ${EndIf}
 
    ; Write updated value
    ${If} $2 == "HKLM"
        WriteRegExpandStr HKLM $5 $1 $6
    ${Else}
        WriteRegExpandStr HKCU $5 $1 $6
    ${EndIf}
 
    StrCpy $0 $6
 
hEnvVarUpdate_done:
    Pop $R0
    Pop $9
    Pop $8
    Pop $7
    Pop $6
    Pop $5
    Pop $4
    Pop $3
    Pop $2
    Pop $1
    Exch $0
FunctionEnd
!macroend
 
!macro EnvVarUpdate RESULT ENVVAR ACTION ROOT VALUE
    Push "${VALUE}"
    Push "${ROOT}"
    Push "${ACTION}"
    Push "${ENVVAR}"
    Call hEnvVarUpdate
    Pop "${RESULT}"
!macroend
 
!macro un.EnvVarUpdate RESULT ENVVAR ACTION ROOT VALUE
    Push "${VALUE}"
    Push "${ROOT}"
    Push "${ACTION}"
    Push "${ENVVAR}"
    Call un.hEnvVarUpdate
    Pop "${RESULT}"
!macroend
 
!insertmacro hEnvVarUpdate ""
!insertmacro hEnvVarUpdate "un."
 
!define EnvVarUpdate '!insertmacro EnvVarUpdate'
!define un.EnvVarUpdate '!insertmacro un.EnvVarUpdate'
 
!endif
