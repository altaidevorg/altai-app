/**
 * Map raw host / RPC error codes to short user-facing copy (A6.111).
 * Host wires may still log the raw code in Extension Host channel.
 */

const HOST_USER_MESSAGES: Record<string, string> = {
  journal_unavailable:
    "Chat history is unavailable on this host (session journal could not open). You can still start a local turn; history will appear once the host journal is healthy.",
  unsupported_config_patch:
    "This setting cannot be saved by the agent host. Check altai CLI / config support, or change the mode again later.",
  unsupported_settings_patch:
    "This settings change is not supported by the VS Code host adapter.",
  configuration_unavailable:
    "ALTAI configuration could not be read or written in this workspace.",
  invalid_settings_response:
    "The host returned an unexpected settings payload.",
  permission_bypass_requires_confirmation:
    "Bypass mode needs explicit confirmation and is not available from this control.",
  host_not_ready: "ALTAI host is not ready yet.",
  workspace_not_trusted:
    "This workspace is not trusted. Trust it before ALTAI can attach files or start agents.",
  workspace_untrusted:
    "This workspace is not trusted. Trust it before ALTAI can attach files or start agents.",
  invalid_permission: "That permission mode is not valid for this host.",
  invalid_model: "That model id is not valid for this host.",
  api_key_not_configured:
    "No API key for the active provider. Open Settings → Models → API keys and Connect.",
  provider_connection_cancelled: "Provider connection was cancelled.",
  invalid_provider_connection:
    "Could not connect that provider. Check the provider id and try again.",
};

/**
 * Extract a stable error code from thrown values (Error.message or plain string).
 */
export function extractHostErrorCode(error: unknown): string {
  const raw =
    error instanceof Error
      ? error.message
      : typeof error === "string"
        ? error
        : String(error ?? "");
  const trimmed = raw.trim();
  if (!trimmed) {
    return "unknown";
  }
  // JSON-RPC style: { code, message: "journal_unavailable" } already stringified
  const snake = trimmed.match(/\b([a-z][a-z0-9]*(?:_[a-z0-9]+)+)\b/);
  if (snake?.[1] && HOST_USER_MESSAGES[snake[1]]) {
    return snake[1];
  }
  if (HOST_USER_MESSAGES[trimmed]) {
    return trimmed;
  }
  return trimmed;
}

/**
 * User-facing message for a host error. Falls back to a cleaned raw message.
 */
export function formatHostUserError(error: unknown): string {
  const code = extractHostErrorCode(error);
  if (HOST_USER_MESSAGES[code]) {
    return HOST_USER_MESSAGES[code]!;
  }
  // Avoid dumping huge stacks into the panel.
  if (code.length > 180) {
    return `${code.slice(0, 177)}…`;
  }
  // Prefer not to show snake_case codes raw when they look like codes.
  if (/^[a-z][a-z0-9]*(?:_[a-z0-9]+)+$/.test(code)) {
    return `Something went wrong (${code.replace(/_/g, " ")}). Check ALTAI logs.`;
  }
  return code;
}

export function isJournalUnavailableError(error: unknown): boolean {
  return extractHostErrorCode(error) === "journal_unavailable";
}
