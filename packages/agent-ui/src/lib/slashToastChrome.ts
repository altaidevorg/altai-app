/**
 * Pure slash-command toast copy (A6.179).
 */

export function switchedAgentToast(agentName: string): string {
  return `Switched to ${agentName}`;
}

export function agentSettingsToast(hadAgentQuery: boolean): string {
  return hadAgentQuery
    ? "Agent not found; opened agent settings"
    : "Opened agent settings";
}
