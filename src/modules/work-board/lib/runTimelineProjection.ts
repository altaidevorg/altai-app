import type { WorkEvent } from "@altai/host-contract";

/**
 * Run timeline projection (package 063, PR 1). The Work store already
 * records every transition in the same transaction as the mutation
 * (`work_events`); this projection turns that log into the rows a Run
 * Inspector renders. Rows keep label and detail separate — the detail is
 * the transition's typed fact (from/to states, terminal phase, run id),
 * and it stays null when the event carries none, never placeholder text.
 */

type Payload = Record<string, unknown> | null;

export type WorkTimelineRow = {
  id: number;
  label: string;
  detail: string | null;
  atMs: number;
};

function parsePayload(raw: string): Payload {
  try {
    const parsed: unknown = JSON.parse(raw);
    return typeof parsed === "object" && parsed !== null
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

function str(payload: Payload, key: string): string | null {
  const value = payload?.[key];
  return typeof value === "string" && value.length > 0 ? value : null;
}

function num(payload: Payload, key: string): number | null {
  const value = payload?.[key];
  return typeof value === "number" ? value : null;
}

function label(value: string): string {
  return value.replace(/_/g, " ");
}

function toRow(event: WorkEvent): WorkTimelineRow {
  const payload = parsePayload(event.payloadJson);
  switch (event.kind) {
    case "created":
      return { id: event.id, label: "Created", detail: null, atMs: event.createdAtMs };
    case "state_changed": {
      const from = str(payload, "from");
      const to = str(payload, "to");
      return {
        id: event.id,
        label: "State changed",
        detail: from && to ? `${label(from)} → ${label(to)}` : null,
        atMs: event.createdAtMs,
      };
    }
    case "attempt_started": {
      const number = num(payload, "number");
      return {
        id: event.id,
        label: number ? `Attempt ${number} started` : "Attempt started",
        detail: null,
        atMs: event.createdAtMs,
      };
    }
    case "attempt_run_bound": {
      const runId = str(payload, "runId");
      return {
        id: event.id,
        label: "Run bound",
        detail: runId,
        atMs: event.createdAtMs,
      };
    }
    case "attempt_finished": {
      const phase = str(payload, "phase");
      return {
        id: event.id,
        label: phase ? `Attempt ${label(phase)}` : "Attempt finished",
        detail: null,
        atMs: event.createdAtMs,
      };
    }
    case "accepted":
      return { id: event.id, label: "Accepted", detail: null, atMs: event.createdAtMs };
    case "returned":
      return { id: event.id, label: "Returned", detail: null, atMs: event.createdAtMs };
    default:
      return {
        id: event.id,
        label: label(event.kind),
        detail: null,
        atMs: event.createdAtMs,
      };
  }
}

/** Project the store's transition log (oldest first) into timeline rows,
 *  preserving order. */
export function toWorkTimeline(events: readonly WorkEvent[]): WorkTimelineRow[] {
  return events.map(toRow);
}
