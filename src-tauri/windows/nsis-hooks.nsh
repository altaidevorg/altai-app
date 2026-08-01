!include "WinMessages.nsh"

; Keep ALTAI's per-user install directory on PATH without duplicating it.
; PowerShell is available on every supported Windows 10/11 installation and
; lets us update the user environment without truncating long PATH values.
!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$target = [IO.Path]::GetFullPath(''$INSTDIR'').TrimEnd(''\''); $$parts = @([Environment]::GetEnvironmentVariable(''Path'', ''User'') -split '';'' | Where-Object { $$_ }); if (-not ($$parts | Where-Object { [StringComparer]::OrdinalIgnoreCase.Equals($$_.TrimEnd(''\''), $$target) })) { [Environment]::SetEnvironmentVariable(''Path'', (($$parts + $$target) -join '';''), ''User'') }"'
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$target = [IO.Path]::GetFullPath(''$INSTDIR'').TrimEnd(''\''); $$parts = @([Environment]::GetEnvironmentVariable(''Path'', ''User'') -split '';'' | Where-Object { $$_ -and -not [StringComparer]::OrdinalIgnoreCase.Equals($$_.TrimEnd(''\''), $$target) }); [Environment]::SetEnvironmentVariable(''Path'', ($$parts -join '';''), ''User'')"'
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend
