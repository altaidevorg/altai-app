!include "WinMessages.nsh"

; Keep ALTAI's per-user install directory on PATH without duplicating it.
; PowerShell is available on every supported Windows 10/11 installation and
; lets us update the user environment without truncating long PATH values.
!macro NSIS_HOOK_POSTINSTALL
  ; Pass the directory through the child-process environment, never by
  ; interpolating it into PowerShell source or its command line. Install paths
  ; are user-controlled and may contain PowerShell metacharacters.
  System::Call 'Kernel32::SetEnvironmentVariable(t "ALTAI_INSTALL_DIR", t "$INSTDIR") i .r0'
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -EncodedCommand JAB0AD0AWwBFAG4AdgBpAHIAbwBuAG0AZQBuAHQAXQA6ADoARwBlAHQARQBuAHYAaQByAG8AbgBtAGUAbgB0AFYAYQByAGkAYQBiAGwAZQAoACcAQQBMAFQAQQBJAF8ASQBOAFMAVABBAEwATABfAEQASQBSACcALAAnAFAAcgBvAGMAZQBzAHMAJwApADsAJABwAD0AWwBFAG4AdgBpAHIAbwBuAG0AZQBuAHQAXQA6ADoARwBlAHQARQBuAHYAaQByAG8AbgBtAGUAbgB0AFYAYQByAGkAYQBiAGwAZQAoACcAUABhAHQAaAAnACwAJwBVAHMAZQByACcAKQA7AGkAZgAoAC0AbgBvAHQAIAAoACgAJABwACAALQBzAHAAbABpAHQAIAAnADsAJwApACAALQBjAG8AbgB0AGEAaQBuAHMAIAAkAHQAKQApAHsAWwBFAG4AdgBpAHIAbwBuAG0AZQBuAHQAXQA6ADoAUwBlAHQARQBuAHYAaQByAG8AbgBtAGUAbgB0AFYAYQByAGkAYQBiAGwAZQAoACcAUABhAHQAaAAnACwAKAAkAHAALAAkAHQAIAAtAGoAbwBpAG4AIAAnADsAJwApACwAJwBVAHMAZQByACcAKQB9AA=='
  ; Do not synchronously broadcast to every top-level window. A hung window can
  ; otherwise block the installer (including unattended installs) for minutes.
  ; SendNotifyMessage returns immediately for other processes while preserving
  ; the standard environment-change notification for new shells.
  System::Call 'user32::SendNotifyMessage(p ${HWND_BROADCAST}, i ${WM_SETTINGCHANGE}, p 0, t "Environment") i .r0'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  System::Call 'Kernel32::SetEnvironmentVariable(t "ALTAI_INSTALL_DIR", t "$INSTDIR") i .r0'
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -EncodedCommand JAB0AD0AWwBFAG4AdgBpAHIAbwBuAG0AZQBuAHQAXQA6ADoARwBlAHQARQBuAHYAaQByAG8AbgBtAGUAbgB0AFYAYQByAGkAYQBiAGwAZQAoACcAQQBMAFQAQQBJAF8ASQBOAFMAVABBAEwATABfAEQASQBSACcALAAnAFAAcgBvAGMAZQBzAHMAJwApADsAJABwAD0AWwBFAG4AdgBpAHIAbwBuAG0AZQBuAHQAXQA6ADoARwBlAHQARQBuAHYAaQByAG8AbgBtAGUAbgB0AFYAYQByAGkAYQBiAGwAZQAoACcAUABhAHQAaAAnACwAJwBVAHMAZQByACcAKQA7AFsARQBuAHYAaQByAG8AbgBtAGUAbgB0AF0AOgA6AFMAZQB0AEUAbgB2AGkAcgBvAG4AbQBlAG4AdABWAGEAcgBpAGEAYgBsAGUAKAAnAFAAYQB0AGgAJwAsACgAKAAkAHAAIAAtAHMAcABsAGkAdAAgACcAOwAnAHwAVwBoAGUAcgBlAC0ATwBiAGoAZQBjAHQAIAB7ACQAXwAgAC0AbgBlACAAJAB0AH0AKQAgAC0AagBvAGkAbgAgACcAOwAnACkALAAnAFUAcwBlAHIAJwApAA=='
  ; See the post-install hook: this must not let an unrelated hung window block
  ; an unattended uninstallation.
  System::Call 'user32::SendNotifyMessage(p ${HWND_BROADCAST}, i ${WM_SETTINGCHANGE}, p 0, t "Environment") i .r0'
!macroend
