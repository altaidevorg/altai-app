/**
 * Pure mapping from composer submit plan → host runtime intent (A6.35).
 * Hosts own I/O (startRun / steer / toast); this only chooses the port call.
 */

import type { ComposerAction } from "./composerEnterAction.js";
import type { ComposerMultimodalParts } from "./composerSubmitCompose.js";
import type { ComposerSubmitPlan } from "./composerSubmitPlan.js";

export type ComposerSubmitHostContext = {
  sessionId: string | null | undefined;
  runId: string | null | undefined;
};

export type ComposerSubmitHostIntent =
  | { kind: "noop" }
  | {
      kind: "handled";
      clearDraft: true;
      toast?: string;
    }
  | {
      kind: "steer";
      sessionId: string;
      runId: string;
      composed: string;
      multimodal: ComposerMultimodalParts;
      clearDraftOnAccept: true;
    }
  | {
      kind: "send";
      action: Extract<ComposerAction, "send" | "queue">;
      sessionId: string;
      composed: string;
      multimodal: ComposerMultimodalParts;
      queue: boolean;
      clearDraftOnAccept: true;
    };

/**
 * Translate a pure submit plan into a concrete HostPorts-shaped intent.
 * Missing session/run ids become noop (same as Desktop early-return).
 */
export function mapComposerSubmitPlanToHostIntent(
  plan: ComposerSubmitPlan,
  context: ComposerSubmitHostContext,
): ComposerSubmitHostIntent {
  if (plan.kind === "noop") return { kind: "noop" };
  if (plan.kind === "handled") {
    return {
      kind: "handled",
      clearDraft: true,
      ...(plan.toast ? { toast: plan.toast } : {}),
    };
  }

  const sessionId = context.sessionId;
  if (!sessionId) return { kind: "noop" };

  if (plan.action === "steer") {
    const runId = context.runId;
    if (!runId) return { kind: "noop" };
    return {
      kind: "steer",
      sessionId,
      runId,
      composed: plan.composed,
      multimodal: plan.multimodal,
      clearDraftOnAccept: true,
    };
  }

  return {
    kind: "send",
    action: plan.action,
    sessionId,
    composed: plan.composed,
    multimodal: plan.multimodal,
    queue: plan.action === "queue",
    clearDraftOnAccept: true,
  };
}
