!include "WinMessages.nsh"

; Keep ALTAI's per-user install directory on PATH without duplicating it.
; PowerShell is available on every supported Windows 10/11 installation and
; lets us update the user environment without truncating long PATH values.
!macro NSIS_HOOK_POSTINSTALL
  ; Pass the directory through the child-process environment, never by
  ; interpolating it into PowerShell source or its command line. Install paths
  ; are user-controlled and may contain PowerShell metacharacters.
  System::Call 'Kernel32::SetEnvironmentVariable(t "ALTAI_INSTALL_DIR", t "$INSTDIR") i .r0'
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -Command "$$target = [IO.Path]::GetFullPath([Environment]::GetEnvironmentVariable(''ALTAI_INSTALL_DIR'', ''Process'')).TrimEnd(''\''); $$parts = @([Environment]::GetEnvironmentVariable(''Path'', ''User'') -split '';'' | Where-Object { $$_ }); if (-not ($$parts | Where-Object { [StringComparer]::OrdinalIgnoreCase.Equals($$_.TrimEnd(''\''), $$target) })) { [Environment]::SetEnvironmentVariable(''Path'', (($$parts + $$target) -join '';''), ''User'') }"'
  ; Do not synchronously broadcast to every top-level window. A hung window can
  ; otherwise block the installer (including unattended installs) for minutes.
  ; SendNotifyMessage returns immediately for other processes while preserving
  ; the standard environment-change notification for new shells.
  System::Call 'user32::SendNotifyMessage(p ${HWND_BROADCAST}, i ${WM_SETTINGCHANGE}, p 0, t "Environment") i .r0'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  System::Call 'Kernel32::SetEnvironmentVariable(t "ALTAI_INSTALL_DIR", t "$INSTDIR") i .r0'
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -Command "$$target = [IO.Path]::GetFullPath([Environment]::GetEnvironmentVariable(''ALTAI_INSTALL_DIR'', ''Process'')).TrimEnd(''\''); $$parts = @([Environment]::GetEnvironmentVariable(''Path'', ''User'') -split '';'' | Where-Object { $$_ -and -not [StringComparer]::OrdinalIgnoreCase.Equals($$_.TrimEnd(''\''), $$target) }); [Environment]::SetEnvironmentVariable(''Path'', ($$parts -join '';''), ''User'')"'
  ; See the post-install hook: this must not let an unrelated hung window block
  ; an unattended uninstallation.
  System::Call 'user32::SendNotifyMessage(p ${HWND_BROADCAST}, i ${WM_SETTINGCHANGE}, p 0, t "Environment") i .r0'
!macroend
