; Custom NSIS hooks for Moonton Claude Desktop
; Add mcd-cli to user PATH during installation

!include "WinMessages.nsh"
!include "WordFunc.nsh"

; Called after installation completes
!macro NSIS_HOOK_POSTINSTALL
  ; Add mcd-cli directory to user PATH
  ReadRegStr $0 HKCU "Environment" "Path"
  ${If} $0 != ""
    ; Check if path already contains mcd-cli using simple string search
    StrCpy $1 "$0" "" 0
    ${WordFind} $1 "$INSTDIR\resources\mcd-cli" "E+1{" $2
    IfErrors 0 +4
      ; Path not found, add it
      WriteRegExpandStr HKCU "Environment" "Path" "$0;$INSTDIR\resources\mcd-cli"
      DetailPrint "Added mcd-cli to user PATH"
      Goto done_postinstall
    DetailPrint "mcd-cli already in user PATH"
    done_postinstall:
  ${Else}
    ; PATH is empty, set it
    WriteRegExpandStr HKCU "Environment" "Path" "$INSTDIR\resources\mcd-cli"
    DetailPrint "Set user PATH to mcd-cli"
  ${EndIf}

  ; Broadcast environment change to all windows
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend

; Called before uninstallation
!macro NSIS_HOOK_PREUNINSTALL
  ; Remove mcd-cli from user PATH
  ReadRegStr $0 HKCU "Environment" "Path"
  ${If} $0 != ""
    ; Remove the mcd-cli path (handle both ;path and path; cases)
    ${WordReplace} $0 ";$INSTDIR\resources\mcd-cli" "" "+" $1
    ${WordReplace} $1 "$INSTDIR\resources\mcd-cli;" "" "+" $2
    ${WordReplace} $2 "$INSTDIR\resources\mcd-cli" "" "+" $3

    ; Only update if changed
    ${If} $3 != $0
      WriteRegExpandStr HKCU "Environment" "Path" "$3"
      DetailPrint "Removed mcd-cli from user PATH"

      ; Broadcast environment change
      SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
    ${EndIf}
  ${EndIf}
!macroend
