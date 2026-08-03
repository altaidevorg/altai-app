; Keep ALTAI's per-user install directory on PATH without duplicating it.
; PowerShell is available on every supported Windows 10/11 installation and
; lets us update the user environment without truncating long PATH values.
!macro NSIS_HOOK_POSTINSTALL
  ; Pass the directory through the child-process environment, never by
  ; interpolating it into PowerShell source or its command line. Install paths
  ; are user-controlled and may contain PowerShell metacharacters.
  System::Call 'Kernel32::SetEnvironmentVariable(t "ALTAI_INSTALL_DIR", t "$INSTDIR") i .r0'
  Exec '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -EncodedCommand JAB0AD0AWwBFAG4AdgBpAHIAbwBuAG0AZQBuAHQAXQA6ADoARwBlAHQARQBuAHYAaQByAG8AbgBtAGUAbgB0AFYAYQByAGkAYQBiAGwAZQAoACcAQQBMAFQAQQBJAF8ASQBOAFMAVABBAEwATABfAEQASQBSACcALAAnAFAAcgBvAGMAZQBzAHMAJwApADsAJABwAD0AWwBFAG4AdgBpAHIAbwBuAG0AZQBuAHQAXQA6ADoARwBlAHQARQBuAHYAaQByAG8AbgBtAGUAbgB0AFYAYQByAGkAYQBiAGwAZQAoACcAUABhAHQAaAAnACwAJwBVAHMAZQByACcAKQA7AGkAZgAoAC0AbgBvAHQAIAAoACgAJABwACAALQBzAHAAbABpAHQAIAAnADsAJwApACAALQBjAG8AbgB0AGEAaQBuAHMAIAAkAHQAKQApAHsAWwBFAG4AdgBpAHIAbwBuAG0AZQBuAHQAXQA6ADoAUwBlAHQARQBuAHYAaQByAG8AbgBtAGUAbgB0AFYAYQByAGkAYQBiAGwAZQAoACcAUABhAHQAaAAnACwAKAAkAHAALAAkAHQAIAAtAGoAbwBpAG4AIAAnADsAJwApACwAJwBVAHMAZQByACcAKQB9AA=='
  ; The persisted user PATH is read by newly opened terminals. Do not broadcast
  ; WM_SETTINGCHANGE here: an unrelated top-level window can block an unattended
  ; NSIS installation indefinitely.
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  System::Call 'Kernel32::SetEnvironmentVariable(t "ALTAI_INSTALL_DIR", t "$INSTDIR") i .r0'
  Exec '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -EncodedCommand JAB0AD0AWwBFAG4AdgBpAHIAbwBuAG0AZQBuAHQAXQA6ADoARwBlAHQARQBuAHYAaQByAG8AbgBtAGUAbgB0AFYAYQByAGkAYQBiAGwAZQAoACcAQQBMAFQAQQBJAF8ASQBOAFMAVABBAEwATABfAEQASQBSACcALAAnAFAAcgBvAGMAZQBzAHMAJwApADsAJABwAD0AWwBFAG4AdgBpAHIAbwBuAG0AZQBuAHQAXQA6ADoARwBlAHQARQBuAHYAaQByAG8AbgBtAGUAbgB0AFYAYQByAGkAYQBiAGwAZQAoACcAUABhAHQAaAAnACwAJwBVAHMAZQByACcAKQA7AFsARQBuAHYAaQByAG8AbgBtAGUAbgB0AF0AOgA6AFMAZQB0AEUAbgB2AGkAcgBvAG4AbQBlAG4AdABWAGEAcgBpAGEAYgBsAGUAKAAnAFAAYQB0AGgAJwAsACgAKAAkAHAAIAAtAHMAcABsAGkAdAAgACcAOwAnAHwAVwBoAGUAcgBlAC0ATwBiAGoAZQBjAHQAIAB7ACQAXwAgAC0AbgBlACAAJAB0AH0AKQAgAC0AagBvAGkAbgAgACcAOwAnACkALAAnAFUAcwBlAHIAJwApAA=='
  ; New terminals read the persisted user PATH; avoid a broadcast that can block
  ; unattended uninstallation on an unrelated window.
!macroend
