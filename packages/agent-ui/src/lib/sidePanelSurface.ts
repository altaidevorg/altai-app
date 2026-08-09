/**
 * Pure side-panel chrome surface + open-chat tab policy (A6.134).
 * Hosts own session stores and storage I/O.
 */

export type SidePanelChromeSurface = "history" | "inspector" | null;

/**
 * Toggle a history/inspector destination. Activating another surface (or the same)
 * never invents Work/Inbox overlays — those route to Operations in hosts.
 */
export function toggleSidePanelChromeSurface(
  current: SidePanelChromeSurface,
  target: Exclude<SidePanelChromeSurface, null>,
): SidePanelChromeSurface {
  return current === target ? null : target;
}

export type SidePanelOpenEventDetail = {
  surface?: string;
  view?: "runs" | "scheduled";
};

export type SidePanelOpenResolution =
  | { kind: "surface"; surface: Exclude<SidePanelChromeSurface, null> }
  | { kind: "review" }
  | { kind: "operations"; view: "work" | "inbox" | "runs" | "overview"; workHubView?: "runs" | "scheduled" }
  | { kind: "ignore" };

/**
 * Map legacy `altai:open-ai-surface` details onto history/inspector or Operations.
 */
export function resolveSidePanelOpenEvent(
  detail: SidePanelOpenEventDetail | undefined,
): SidePanelOpenResolution {
  const surface = detail?.surface;
  if (surface === "review") {
    return { kind: "review" };
  }
  if (surface === "history" || surface === "inspector") {
    return { kind: "surface", surface };
  }
  if (surface === "inbox") {
    return { kind: "operations", view: "inbox" };
  }
  if (
    surface === "work" ||
    surface === "tasks" ||
    surface === "automations"
  ) {
    const scheduled =
      surface === "automations" || detail?.view === "scheduled";
    return {
      kind: "operations",
      view: "work",
      workHubView: scheduled ? "scheduled" : "runs",
    };
  }
  return { kind: "ignore" };
}

/** Keep tab ids that still exist; ensure active session is opened as a tab. */
export function reconcileOpenChatTabIds(input: {
  openIds: readonly string[];
  sessionIds: readonly string[];
  activeSessionId: string | null | undefined;
}): string[] {
  const valid = input.openIds.filter((id) => input.sessionIds.includes(id));
  if (input.activeSessionId && !valid.includes(input.activeSessionId)) {
    valid.push(input.activeSessionId);
  }
  return valid;
}

/**
 * Close a chat tab. Host supplies `createSessionId` when the last tab would close
 * (so the store can create a real session).
 */
export function closeChatTabSelection(input: {
  openIds: readonly string[];
  closingId: string;
  activeSessionId: string | null | undefined;
  createSessionId: () => string;
}): {
  openIds: string[];
  focusSessionId: string | null;
  /** When true, host should create no focus change beyond openIds. */
  closedOnly: boolean;
} {
  const index = input.openIds.indexOf(input.closingId);
  if (index < 0) {
    return {
      openIds: [...input.openIds],
      focusSessionId: null,
      closedOnly: true,
    };
  }
  const remaining = input.openIds.filter((id) => id !== input.closingId);
  if (remaining.length === 0) {
    const id = input.createSessionId();
    return { openIds: [id], focusSessionId: id, closedOnly: false };
  }
  if (input.activeSessionId === input.closingId) {
    const next = remaining[Math.min(index, remaining.length - 1)]!;
    return {
      openIds: [...remaining],
      focusSessionId: next,
      closedOnly: false,
    };
  }
  return {
    openIds: [...remaining],
    focusSessionId: null,
    closedOnly: true,
  };
}

/**
 * After creating a new session, append its tab and clear chrome overlays.
 */
export function openIdsAfterNewChat(
  openIds: readonly string[],
  newId: string,
): string[] {
  return openIds.includes(newId) ? [...openIds] : [...openIds, newId];
}
