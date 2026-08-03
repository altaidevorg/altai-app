/**
 * Typed errors for control-plane operations.
 *
 * All errors are typed, never string matching. The `ControlErrorCode`
 * provides a stable numeric code for protocol-level error reporting.
 */

import { IdError } from "./ids.js";

export enum ControlErrorCode {
  InvalidId = 1,
  StaleRevision = 2,
  NotFound = 3,
  Unauthorized = 4,
  PolicyDenied = 5,
  BudgetStopped = 6,
  Blocked = 7,
  PayloadTooLarge = 8,
  Conflict = 9,
  AlreadyTerminal = 10,
  InternalError = 99,
}

export type ControlError =
  | { kind: "invalid_id"; error: IdError }
  | { kind: "stale_revision"; expected: number; got: number }
  | { kind: "not_found"; entity: string; id: string }
  | { kind: "unauthorized"; actor: string; action: string }
  | { kind: "policy_denied"; reason: string }
  | { kind: "budget_stopped"; scope: string }
  | { kind: "blocked"; blocker_id: string }
  | { kind: "payload_too_large"; max_bytes: number; actual_bytes: number }
  | { kind: "conflict"; reason: string }
  | { kind: "already_terminal"; entity: string; id: string }
  | { kind: "internal_error"; reason: string };

export function controlErrorCode(err: ControlError): ControlErrorCode {
  switch (err.kind) {
    case "invalid_id":
      return ControlErrorCode.InvalidId;
    case "stale_revision":
      return ControlErrorCode.StaleRevision;
    case "not_found":
      return ControlErrorCode.NotFound;
    case "unauthorized":
      return ControlErrorCode.Unauthorized;
    case "policy_denied":
      return ControlErrorCode.PolicyDenied;
    case "budget_stopped":
      return ControlErrorCode.BudgetStopped;
    case "blocked":
      return ControlErrorCode.Blocked;
    case "payload_too_large":
      return ControlErrorCode.PayloadTooLarge;
    case "conflict":
      return ControlErrorCode.Conflict;
    case "already_terminal":
      return ControlErrorCode.AlreadyTerminal;
    case "internal_error":
      return ControlErrorCode.InternalError;
  }
}

export function controlErrorMessage(err: ControlError): string {
  switch (err.kind) {
    case "invalid_id":
      return `invalid id: ${err.error.kind}`;
    case "stale_revision":
      return `stale revision: expected ${err.expected}, got ${err.got}`;
    case "not_found":
      return `${err.entity} not found: ${err.id}`;
    case "unauthorized":
      return `unauthorized: ${err.actor} cannot ${err.action}`;
    case "policy_denied":
      return `policy denied: ${err.reason}`;
    case "budget_stopped":
      return `budget stopped: ${err.scope}`;
    case "blocked":
      return `blocked by: ${err.blocker_id}`;
    case "payload_too_large":
      return `payload too large: ${err.actual_bytes} > ${err.max_bytes}`;
    case "conflict":
      return `conflict: ${err.reason}`;
    case "already_terminal":
      return `${err.entity} already terminal: ${err.id}`;
    case "internal_error":
      return `internal error: ${err.reason}`;
  }
}

export function idErrorToControlError(error: IdError): ControlError {
  return { kind: "invalid_id", error };
}
