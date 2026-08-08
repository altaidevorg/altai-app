/**
 * Pure helpers for MCP status chrome (A6.81).
 */

export type McpServerView = {
  id: string;
  name: string;
  enabled: boolean;
  connected: boolean;
  error?: string;
};

export function canMountMcpStatus(flags: { mcpList: boolean }): boolean {
  return flags.mcpList;
}

export function sortMcpServersForDisplay(
  servers: readonly McpServerView[],
): McpServerView[] {
  return [...servers].sort((a, b) => {
    const score = (s: McpServerView): number => {
      if (s.error) {
        return 0;
      }
      if (!s.connected) {
        return 1;
      }
      if (!s.enabled) {
        return 2;
      }
      return 3;
    };
    const delta = score(a) - score(b);
    if (delta !== 0) {
      return delta;
    }
    return a.name.localeCompare(b.name);
  });
}

export function mcpServerStatusCopy(server: McpServerView): string {
  if (server.error?.trim()) {
    return server.error.trim();
  }
  if (!server.enabled) {
    return "Disabled";
  }
  return server.connected ? "Connected" : "Disconnected";
}

export function mcpSummaryCopy(servers: readonly McpServerView[]): string {
  if (servers.length === 0) {
    return "No MCP servers";
  }
  const connected = servers.filter((s) => s.connected && s.enabled).length;
  return `${connected}/${servers.length} MCP connected`;
}
