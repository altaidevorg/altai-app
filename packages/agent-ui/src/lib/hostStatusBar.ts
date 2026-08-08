/**
 * Pure presentation for the host lifecycle status-bar badge (A6.119).
 * No vscode imports.
 */

export type HostStatusBarInput = {
  status: string;
  message: string;
  diagnosticCode?: string;
};

export type HostStatusBarPresentation = {
  show: boolean;
  text: string;
  tooltip: string;
  command: string;
  warning: boolean;
};

function actionForDiagnostic(code: string | undefined): {
  command: string;
  actionLabel: string;
} {
  const normalized = code?.trim() ?? "";
  if (normalized === "host.untrusted") {
    return {
      command: "workbench.action.manageWorkspaceTrust",
      actionLabel: "Manage Trust",
    };
  }
  if (normalized === "host.missing") {
    return {
      command: "altai.openExtensionSettings",
      actionLabel: "Host Path Settings",
    };
  }
  if (normalized === "host.no_workspace") {
    return {
      command: "workbench.action.files.openFolder",
      actionLabel: "Open Folder",
    };
  }
  if (normalized === "host.virtual_workspace") {
    return {
      command: "workbench.action.files.openFolder",
      actionLabel: "Open Folder",
    };
  }
  return {
    command: "altai.runDiagnostics",
    actionLabel: "Run Diagnostics",
  };
}

export function hostStatusBarPresentation(
  status: HostStatusBarInput,
): HostStatusBarPresentation {
  const message = status.message.trim() || status.status;
  if (status.status === "ready") {
    return {
      show: false,
      text: "$(check) ALTAI host",
      tooltip: message,
      command: "altai.openSidePanel",
      warning: false,
    };
  }
  if (status.status === "connecting") {
    return {
      show: true,
      text: "$(sync~spin) ALTAI host",
      tooltip: message,
      command: "altai.openSidePanel",
      warning: false,
    };
  }
  if (status.status === "error") {
    const detail = status.diagnosticCode
      ? `${message} (${status.diagnosticCode})`
      : message;
    const action = actionForDiagnostic(status.diagnosticCode);
    return {
      show: true,
      text: "$(error) ALTAI host",
      tooltip: `${detail} — ${action.actionLabel}`,
      command: action.command,
      warning: true,
    };
  }
  // disconnected (or other non-ready): surface recovery when a diagnostic is set
  if (status.diagnosticCode) {
    const detail = `${message} (${status.diagnosticCode})`;
    const action = actionForDiagnostic(status.diagnosticCode);
    return {
      show: true,
      text: "$(debug-disconnect) ALTAI host",
      tooltip: `${detail} — ${action.actionLabel}`,
      command: action.command,
      warning: status.diagnosticCode === "host.untrusted",
    };
  }
  return {
    show: true,
    text: "$(debug-disconnect) ALTAI host",
    tooltip: `${message} — Open ALTAI`,
    command: "altai.openSidePanel",
    warning: false,
  };
}
