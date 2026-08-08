/**
 * Allowlisted recovery / diagnostic commands hosts may invoke (A6.106).
 * VS Code ids are host-agnostic string tokens; Desktop may map or gate them.
 */

export const ALTAI_RECOVERY_COMMANDS = [
  "altai.openLogs",
  "altai.runDiagnostics",
  "altai.copyDiagnosticsReport",
  "altai.restartAgentHost",
  "altai.showVersionCompatibility",
  "altai.connectProvider",
  "altai.clearProviderCredential",
  "altai.openWalkthrough",
  "altai.openExtensionSettings",
  "altai.askAboutProblems",
  "altai.pickProjectRoot",
  "workbench.action.manageWorkspaceTrust",
  "workbench.action.files.openFolder",
  "workbench.action.openGlobalKeybindings",
  "workbench.action.selectTheme",
  "workbench.action.openSettings",
] as const;

export type AltaiRecoveryCommand = (typeof ALTAI_RECOVERY_COMMANDS)[number];

export type RecoveryAction = {
  command: AltaiRecoveryCommand;
  label: string;
};

export function isAltaiRecoveryCommand(
  value: string,
): value is AltaiRecoveryCommand {
  return (ALTAI_RECOVERY_COMMANDS as readonly string[]).includes(value);
}

/**
 * Wait-shell / Settings recovery buttons.
 * When the host reports `host.untrusted`, surface Manage workspace trust first.
 * Connect/clear stay available via palette and slash; not always-on buttons.
 */
export function listRecoveryActions(input?: {
  diagnosticCode?: string;
}): readonly RecoveryAction[] {
  const actions: RecoveryAction[] = [];
  if (input?.diagnosticCode === "host.untrusted") {
    actions.push({
      command: "workbench.action.manageWorkspaceTrust",
      label: "Manage workspace trust",
    });
  }
  if (input?.diagnosticCode === "host.missing") {
    actions.push({
      command: "altai.openExtensionSettings",
      label: "Host path settings",
    });
  }
  if (input?.diagnosticCode === "host.no_workspace") {
    actions.push({
      command: "workbench.action.files.openFolder",
      label: "Open folder",
    });
  }
  if (input?.diagnosticCode === "host.virtual_workspace") {
    actions.push({
      command: "workbench.action.files.openFolder",
      label: "Open folder",
    });
  }
  actions.push(
    { command: "altai.openLogs", label: "Open logs" },
    { command: "altai.runDiagnostics", label: "Run diagnostics" },
    { command: "altai.copyDiagnosticsReport", label: "Copy diagnostics" },
    { command: "altai.restartAgentHost", label: "Restart host" },
    { command: "altai.showVersionCompatibility", label: "Version" },
    { command: "altai.openWalkthrough", label: "Get started" },
  );
  return actions;
}
