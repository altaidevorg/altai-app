import {
  createCapabilities,
  type Capabilities,
  type CapabilityAvailability,
  type CapabilityEntry,
  type CapabilityId,
} from "./capabilities.js";
import { HOST_CONTRACT_VERSION, PERMISSION_MODES } from "./types.js";

export type ParseResult<T> =
  | { ok: true; value: T }
  | { ok: false; reason: string };

const AVAILABILITIES: ReadonlySet<CapabilityAvailability> = new Set([
  "available",
  "deferred",
  "unsupported",
]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function parseCapabilities(value: unknown): ParseResult<Capabilities> {
  if (!isRecord(value)) {
    return { ok: false, reason: "capabilities_not_object" };
  }
  if (value.contractVersion !== HOST_CONTRACT_VERSION) {
    return { ok: false, reason: "contract_version_mismatch" };
  }
  if (
    typeof value.protocolVersion !== "number" ||
    !Number.isSafeInteger(value.protocolVersion)
  ) {
    return { ok: false, reason: "protocol_version_invalid" };
  }
  if (typeof value.hostName !== "string" || value.hostName.trim().length === 0) {
    return { ok: false, reason: "host_name_invalid" };
  }
  if (
    typeof value.hostVersion !== "string" ||
    value.hostVersion.trim().length === 0
  ) {
    return { ok: false, reason: "host_version_invalid" };
  }
  if (!Array.isArray(value.capabilities)) {
    return { ok: false, reason: "capabilities_not_array" };
  }

  const capabilities: CapabilityEntry[] = [];
  for (const item of value.capabilities) {
    const parsed = parseCapabilityEntry(item);
    if (!parsed.ok) {
      return parsed;
    }
    capabilities.push(parsed.value);
  }

  return {
    ok: true,
    value: {
      contractVersion: HOST_CONTRACT_VERSION,
      protocolVersion: value.protocolVersion,
      hostName: value.hostName,
      hostVersion: value.hostVersion,
      capabilities,
    },
  };
}

function parseCapabilityEntry(
  value: unknown,
): ParseResult<CapabilityEntry> {
  if (!isRecord(value)) {
    return { ok: false, reason: "capability_entry_not_object" };
  }
  if (typeof value.id !== "string" || value.id.trim().length === 0) {
    return { ok: false, reason: "capability_id_invalid" };
  }
  if (
    typeof value.availability !== "string" ||
    !AVAILABILITIES.has(value.availability as CapabilityAvailability)
  ) {
    return { ok: false, reason: "capability_availability_invalid" };
  }

  const entry: CapabilityEntry = {
    id: value.id as CapabilityId,
    availability: value.availability as CapabilityAvailability,
  };
  if (typeof value.note === "string") {
    entry.note = value.note;
  }
  return { ok: true, value: entry };
}

export function parsePermissionMode(
  value: unknown,
): ParseResult<(typeof PERMISSION_MODES)[number]> {
  if (
    typeof value === "string" &&
    (PERMISSION_MODES as readonly string[]).includes(value)
  ) {
    return {
      ok: true,
      value: value as (typeof PERMISSION_MODES)[number],
    };
  }
  return { ok: false, reason: "permission_mode_invalid" };
}

/** Convenience helper for hosts that only need a validated baseline document. */
export function parseOrCreateBaselineCapabilities(input: {
  protocolVersion: number;
  hostName: string;
  hostVersion: string;
  raw?: unknown;
}): ParseResult<Capabilities> {
  if (input.raw === undefined) {
    return {
      ok: true,
      value: createCapabilities({
        protocolVersion: input.protocolVersion,
        hostName: input.hostName,
        hostVersion: input.hostVersion,
      }),
    };
  }
  return parseCapabilities(input.raw);
}
