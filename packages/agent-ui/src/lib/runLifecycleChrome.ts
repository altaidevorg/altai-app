/**
 * Pure helpers for Chat run blocked / warning / recovery chrome.
 * Wave 4 / A6.14 — host maps event payloads; hosts render shared banners.
 */

export function shouldShowChangeReviewBanner(queueLen: number): boolean {
  return queueLen > 0;
}

/**
 * Extract a user-facing blocked-run message from lifecycle terminal payloads.
 * Returns null for clean successes and unrelated events.
 */
export function runBlockedMessageFromEvent(payload: unknown): string | null {
  if (!isRecord(payload)) {
    return null;
  }
  const body = unwrap(payload);
  const crateType =
    (typeof body.type === "string" && body.type) ||
    (isRecord(body.event) &&
      typeof body.event.type === "string" &&
      body.event.type) ||
    "";

  if (
    crateType !== "run_terminated" &&
    crateType !== "run_cancelled" &&
    body.outcome === undefined &&
    body.error === undefined
  ) {
    return null;
  }

  const outcome =
    (typeof body.outcome === "string" && body.outcome) ||
    (isRecord(body.event) &&
      typeof body.event.outcome === "string" &&
      body.event.outcome) ||
    "";

  if (crateType === "run_cancelled" || outcome === "cancelled") {
    return "Run cancelled";
  }

  const errorText =
    (typeof body.error === "string" && body.error) ||
    (isRecord(body.error) &&
      typeof body.error.message === "string" &&
      body.error.message) ||
    (isRecord(body.event) &&
      typeof body.event.error === "string" &&
      body.event.error) ||
    (typeof body.message === "string" && body.message) ||
    "";

  if (
    outcome === "error" ||
    outcome === "failed" ||
    outcome === "failure" ||
    errorText
  ) {
    return errorText.trim() || "Run failed";
  }

  return null;
}

/**
 * Extract an in-run warning string from lifecycle / warning events.
 * Distinct from blocked terminal failures (recovery vs still-running).
 */
export function runWarningMessageFromEvent(
  eventType: string,
  payload: unknown,
): string | null {
  if (eventType === "warning") {
    if (isRecord(payload)) {
      const body = unwrap(payload);
      const text =
        (typeof body.warning === "string" && body.warning) ||
        (typeof body.message === "string" && body.message) ||
        "";
      return text.trim() || "Run warning";
    }
    return "Run warning";
  }
  if (!isRecord(payload)) {
    return null;
  }
  const body = unwrap(payload);
  const crateType =
    (typeof body.type === "string" && body.type) ||
    (isRecord(body.event) &&
      typeof body.event.type === "string" &&
      body.event.type) ||
    "";
  if (crateType === "run_warning") {
    const text =
      (typeof body.warning === "string" && body.warning) ||
      (typeof body.message === "string" && body.message) ||
      (isRecord(body.event) &&
        typeof body.event.warning === "string" &&
        body.event.warning) ||
      "";
    return text.trim() || "Run warning";
  }
  if (crateType === "run_warning_cleared") {
    return "";
  }
  return null;
}

export type RecoveryChromeFlags = {
  blockedMessage: string | null;
  warningMessage: string | null;
  canRetry: boolean;
  canSteer: boolean;
  hasActiveRun: boolean;
};

/** When true, mount shared RunRecoveryActions instead of/in addition to banners. */
export function shouldShowRunRecovery(flags: RecoveryChromeFlags): boolean {
  if (flags.warningMessage) {
    return true;
  }
  if (!flags.blockedMessage) {
    return false;
  }
  return flags.canRetry || flags.canSteer || flags.hasActiveRun;
}

export function recoveryCopy(flags: RecoveryChromeFlags): {
  warning: boolean;
  title: string;
  detail: string;
} {
  if (flags.warningMessage && !flags.blockedMessage) {
    return {
      warning: true,
      title: "Run needs attention",
      detail: flags.warningMessage,
    };
  }
  return {
    warning: false,
    title: "Run blocked",
    detail: flags.blockedMessage ?? "The last run did not complete successfully.",
  };
}

function unwrap(payload: Record<string, unknown>): Record<string, unknown> {
  if (typeof payload.type === "string") {
    return payload;
  }
  if (isRecord(payload.payload) && typeof payload.payload.type === "string") {
    return payload.payload;
  }
  return payload;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
