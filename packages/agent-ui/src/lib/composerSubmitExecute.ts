/**
 * Ports-first composer submit execution (A6.36).
 * Plans + host-intent mapping + injected send/steer I/O — no Tauri/vscode.
 */

import type { ComposerAction, ComposerActionAvailability } from "./composerEnterAction.js";
import type { ComposerSnippet } from "./composerSnippets.js";
import type { ComposerMultimodalParts } from "./composerSubmitCompose.js";
import {
  planComposerSubmit,
  type ComposerSlashResolver,
  type ComposerSubmitSnapshot,
} from "./composerSubmitPlan.js";
import {
  mapComposerSubmitPlanToHostIntent,
  type ComposerSubmitHostIntent,
} from "./composerSubmitHostIntent.js";

export type ComposerSubmitHostHandlers = {
  /** Return true when the host accepted the send/queue (clear draft). */
  send: (input: {
    sessionId: string;
    composed: string;
    multimodal: ComposerMultimodalParts;
    queue: boolean;
  }) => Promise<boolean>;
  /** Return true when the host accepted the steer (clear draft). */
  steer: (input: {
    sessionId: string;
    runId: string;
    composed: string;
    multimodal: ComposerMultimodalParts;
  }) => Promise<boolean>;
  onToast?: (message: string) => void;
  onError?: (input: {
    action: Extract<ComposerAction, "send" | "steer" | "queue">;
    error: unknown;
  }) => void;
};

export type ComposerSubmitExecuteResult =
  | { kind: "noop" }
  | { kind: "handled"; toast?: string }
  | {
      kind: "accepted";
      intent: Extract<ComposerSubmitHostIntent, { kind: "send" | "steer" }>;
    }
  | {
      kind: "rejected";
      intent: Extract<ComposerSubmitHostIntent, { kind: "send" | "steer" }>;
    }
  | {
      kind: "error";
      intent: Extract<ComposerSubmitHostIntent, { kind: "send" | "steer" }>;
      error: unknown;
    };

/**
 * Run plan → host intent → injected send/steer. Pure aside from host I/O.
 */
export async function executeComposerSubmit(input: {
  action: ComposerAction;
  availability: ComposerActionAvailability;
  draft: ComposerSubmitSnapshot;
  catalog: readonly ComposerSnippet[];
  resolveSlash?: ComposerSlashResolver;
  sessionId: string | null | undefined;
  runId: string | null | undefined;
  host: ComposerSubmitHostHandlers;
}): Promise<ComposerSubmitExecuteResult> {
  const plan = planComposerSubmit({
    action: input.action,
    availability: input.availability,
    draft: input.draft,
    catalog: input.catalog,
    resolveSlash: input.resolveSlash,
  });
  const intent = mapComposerSubmitPlanToHostIntent(plan, {
    sessionId: input.sessionId,
    runId: input.runId,
  });

  if (intent.kind === "noop") return { kind: "noop" };
  if (intent.kind === "handled") {
    if (intent.toast) input.host.onToast?.(intent.toast);
    return intent.toast
      ? { kind: "handled", toast: intent.toast }
      : { kind: "handled" };
  }

  try {
    let accepted: boolean;
    if (intent.kind === "steer") {
      accepted = await input.host.steer({
        sessionId: intent.sessionId,
        runId: intent.runId,
        composed: intent.composed,
        multimodal: intent.multimodal,
      });
    } else {
      accepted = await input.host.send({
        sessionId: intent.sessionId,
        composed: intent.composed,
        multimodal: intent.multimodal,
        queue: intent.queue,
      });
    }
    return accepted
      ? { kind: "accepted", intent }
      : { kind: "rejected", intent };
  } catch (error) {
    input.host.onError?.({
      action: intent.kind === "steer" ? "steer" : intent.action,
      error,
    });
    return { kind: "error", intent, error };
  }
}
