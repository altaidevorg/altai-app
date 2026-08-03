/**
 * Event types for the control plane.
 *
 * `ActivityEvent` is the audit-level event: who did what, when, and why.
 * `ControlEvent` is the append-only per-aggregate event used for
 * event-sourced projections and recovery.
 */

import type { Actor } from "./actor.js";
import type { Revision } from "./revision.js";
import type { TypedId } from "./ids.js";

export type EventKind =
  | "created"
  | "updated"
  | "status_changed"
  | "assigned"
  | "wake_requested"
  | "attempt_transitioned"
  | "routine_triggered"
  | "approval_transitioned"
  | "budget_event"
  | "external_sync"
  | "recovery";

/** Audit-level activity event. Appended to `activity_events`. */
export type ActivityEvent = {
  /** Globally unique event ID (ULID). */
  readonly event_id: string;
  /** What kind of activity. */
  readonly kind: EventKind;
  /** Who performed the action. */
  readonly actor: Actor;
  /** When the event occurred (ISO 8601 UTC, supplied by the service clock). */
  readonly timestamp: string;
  /** The organization this event belongs to. */
  readonly organization_id: TypedId;
  /** Optional project scope (null when absent, for byte-identical Rust/TS). */
  readonly project_id: TypedId | null;
  /** Optional work item scope. */
  readonly work_item_id: TypedId | null;
  /** Optional attempt scope. */
  readonly attempt_id: TypedId | null;
  /** Human-readable summary for the audit feed. */
  readonly summary: string;
  /** Correlation ID for tracing across events. */
  readonly correlation_id: string | null;
  /** Causation ID (what caused this event, if any). */
  readonly causation_id: string | null;
};

/** Append-only per-aggregate event. */
export type ControlEvent = {
  /** The aggregate this event belongs to (e.g. "work_item", "attempt"). */
  readonly aggregate: string;
  /** The aggregate's ID as a typed ID struct. */
  readonly aggregate_id: TypedId;
  /** Monotonically increasing sequence within the aggregate. */
  readonly sequence: number;
  /** The event kind. */
  readonly kind: EventKind;
  /** Who caused the event. */
  readonly actor: Actor;
  /** When the event occurred (ISO 8601 UTC). */
  readonly timestamp: string;
  /** The revision of the aggregate after this event. */
  readonly revision: Revision;
  /** The event payload (domain-specific JSON). */
  readonly payload: unknown;
  /** Correlation ID for tracing. */
  readonly correlation_id: string | null;
  /** Causation ID. */
  readonly causation_id: string | null;
};

export const ALL_EVENT_KINDS: readonly EventKind[] = [
  "created",
  "updated",
  "status_changed",
  "assigned",
  "wake_requested",
  "attempt_transitioned",
  "routine_triggered",
  "approval_transitioned",
  "budget_event",
  "external_sync",
  "recovery",
];
