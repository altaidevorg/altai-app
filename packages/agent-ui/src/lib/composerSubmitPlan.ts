/**
 * Pure ports-first composer submit planning (A6.33).
 * Hosts resolve slash commands, then send/steer/queue via HostPorts.runtime.
 */

import type { ComposerAction, ComposerActionAvailability } from "./composerEnterAction.js";
import type { ComposerFileAttachment } from "./composerAttachments.js";
import {
  extractComposerMultimodalParts,
  composeComposerSubmitText,
  type ComposerMultimodalParts,
} from "./composerSubmitCompose.js";
import type { ComposerSnippet } from "./composerSnippets.js";
import {
  applyComposerSlashOutcome,
  buildComposerCommandSource,
  type ComposerSlashOutcome,
} from "./composerDraft.js";
import { hasComposerDraft } from "./composerAttachments.js";

export type ComposerSlashResolver = (
  commandSource: string,
) => ComposerSlashOutcome;

export type ComposerSubmitSnapshot = {
  value: string;
  files: readonly ComposerFileAttachment[];
  snippets: readonly ComposerSnippet[];
  commands: readonly { name: string }[];
};

export type ComposerSubmitPlan =
  | { kind: "noop" }
  | {
      kind: "handled";
      /** Clear composer draft chips/text after local-only slash action. */
      clearDraft: true;
      toast?: string;
    }
  | {
      kind: "submit";
      action: ComposerAction;
      composed: string;
      multimodal: ComposerMultimodalParts;
      /** Host should clear only after send/steer accepted. */
      clearDraftOnAccept: true;
    };

/**
 * Decide whether Enter (or Send) becomes noop, local-handled slash, or a
 * host submit payload. Pure: no I/O, stores, or HostPorts.
 */
export function planComposerSubmit(input: {
  action: ComposerAction;
  availability: ComposerActionAvailability;
  draft: ComposerSubmitSnapshot;
  catalog: readonly ComposerSnippet[];
  resolveSlash?: ComposerSlashResolver;
}): ComposerSubmitPlan {
  const { action, availability, draft } = input;
  if (action === "send" && !availability.canSend) return { kind: "noop" };
  if (action === "steer" && !availability.canSteer) return { kind: "noop" };
  if (action === "queue" && !availability.canQueue) return { kind: "noop" };

  const trimmed = draft.value.trim();
  if (
    !hasComposerDraft({
      value: draft.value,
      files: draft.files,
      snippets: draft.snippets,
      commands: draft.commands,
    })
  ) {
    return { kind: "noop" };
  }

  let effectiveText = trimmed;
  let commandMarker: string | null = null;
  const commandSource = buildComposerCommandSource(
    trimmed,
    draft.commands.map((c) => c.name),
  );

  if (
    input.resolveSlash &&
    (commandSource.startsWith("/") || commandSource.startsWith("#"))
  ) {
    const outcome = input.resolveSlash(commandSource);
    const mapped = applyComposerSlashOutcome(outcome, trimmed);
    if (mapped.abortAsHandled) {
      return {
        kind: "handled",
        clearDraft: true,
        ...(mapped.toast ? { toast: mapped.toast } : {}),
      };
    }
    effectiveText = mapped.effectiveText;
    commandMarker = mapped.commandMarker;
  }

  const composed = composeComposerSubmitText({
    commandMarker,
    effectiveText,
    catalog: input.catalog,
    pickedSnippets: draft.snippets,
    files: draft.files,
  });
  const multimodal = extractComposerMultimodalParts(draft.files);

  return {
    kind: "submit",
    action,
    composed,
    multimodal,
    clearDraftOnAccept: true,
  };
}
