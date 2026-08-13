import type { TypedId } from "./ids.js";

export const CONTROL_PLANE_PROTOCOL_MAJOR = 1;

export type HostCapabilities = { readonly values: readonly string[] };

export type HostRegistration = {
  readonly agent_instance_id: TypedId;
  readonly workspaces: readonly TypedId[];
  readonly capabilities: HostCapabilities;
  readonly protocol_major: number;
};

/** Consumers must redact grant_token from logs and errors. */
export type HostRegistrationRequest = {
  readonly grant_token: string;
  readonly host: HostRegistration;
};

export type RegisteredHost = {
  readonly agent_instance_id: TypedId;
  readonly workspaces: readonly TypedId[];
  readonly capabilities: HostCapabilities;
  readonly registered_at_unix_seconds: number;
};

/** Non-secret control-plane readiness projection. */
export type ControlPlaneHealth = {
  readonly service_version: string;
  readonly protocol_major: number;
  readonly store_kind: string;
  readonly registered_host_count: number;
  readonly database_adapter_ready: boolean;
};
