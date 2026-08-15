import { invoke } from "@tauri-apps/api/core";

/**
 * Health classification for the desktop's control-plane connection
 * (package 061). The probe negotiates capabilities through the same
 * `control_protocol_negotiate` command every host serves (CP-08-45), so
 * "healthy" means: this workspace's `work.db` is served by the shared
 * protocol dispatcher and the client is compatible with it.
 */

/** The protocol version this client speaks (CONTROL_PLANE_PROTOCOL_VERSION 1.0). */
export const OPERATIONS_PROTOCOL_CLIENT_VERSION = { major: 1, minor: 0 } as const;

export const OPERATIONS_CLIENT_NAME = "altai-desktop";

/** Wire shape of `CapabilityNegotiationResponse` (serde snake_case). */
export type OperationsNegotiation = {
  server_version: { major: number; minor: number };
  deployment_mode: string;
  compatible: boolean;
  missing_capabilities: string[];
};

export type OperationsHealth =
  | { connection: "offline"; detail: null }
  | {
      connection: "degraded";
      /** Why the control plane cannot be confirmed healthy. */
      detail: string;
      /** Present when the server answered but is incompatible. */
      negotiation: OperationsNegotiation | null;
    }
  | { connection: "healthy"; negotiation: OperationsNegotiation };

export type NegotiateCommand = (
  workspacePath: string,
  params: unknown,
) => Promise<OperationsNegotiation>;

export function operationsNegotiateParams(): {
  client_version: { major: number; minor: number };
  client_name: string;
  required_capabilities: string[];
} {
  return {
    client_version: { ...OPERATIONS_PROTOCOL_CLIENT_VERSION },
    client_name: OPERATIONS_CLIENT_NAME,
    // PR 1 requires nothing: negotiation alone proves the workspace is served
    // by the shared dispatcher. Later surfaces pin the capabilities they use.
    required_capabilities: [],
  };
}

export async function probeOperationsHealth(input: {
  workspacePath: string | null;
  negotiate: NegotiateCommand;
}): Promise<OperationsHealth> {
  const { workspacePath, negotiate } = input;
  if (workspacePath === null) {
    return { connection: "offline", detail: null };
  }
  try {
    const negotiation = await negotiate(
      workspacePath,
      operationsNegotiateParams(),
    );
    return negotiation.compatible
      ? { connection: "healthy", negotiation }
      : {
          connection: "degraded",
          negotiation,
          detail: degradedDetail(negotiation),
        };
  } catch (error) {
    // The command failed — the workspace may still be usable for other
    // surfaces, but the control plane cannot be confirmed, so the shell must
    // not present it as healthy (e.g. a `work.db` newer than this build).
    return {
      connection: "degraded",
      negotiation: null,
      detail: errorMessage(error),
    };
  }
}

function degradedDetail(negotiation: OperationsNegotiation): string {
  if (negotiation.missing_capabilities.length > 0) {
    return `Server is missing capabilities: ${negotiation.missing_capabilities.join(", ")}`;
  }
  const { major, minor } = negotiation.server_version;
  return `Server protocol version ${major}.${minor} is not compatible with client ${OPERATIONS_PROTOCOL_CLIENT_VERSION.major}.${OPERATIONS_PROTOCOL_CLIENT_VERSION.minor}`;
}

function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  return error instanceof Error ? error.message : String(error);
}

/** The desktop's binding to the CP-08-45 Tauri command. */
export function negotiateViaDesktop(
  workspacePath: string,
  params: unknown,
): Promise<OperationsNegotiation> {
  return invoke<OperationsNegotiation>("control_protocol_negotiate", {
    workspacePath,
    params,
  });
}
