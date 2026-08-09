/**
 * Pure primary IsanAgent target resolution result (A6.178).
 * Host supplies resolved target or missing-key provider id.
 */

import type { ResolvedProviderTarget } from "./fallbackSpec.js";
import { describeUnresolvedIsanAgentTarget } from "./isanagentTargetChrome.js";

export type IsanAgentTargetResolution =
  | { ok: true; target: ResolvedProviderTarget }
  | { ok: false; error: string };

/**
 * Map a resolved target (or null) into the UI ok/error union used on send.
 */
export function toIsanAgentTargetResolution(
  selectedModelId: string,
  target: ResolvedProviderTarget | null,
  knownKeyProvider: string | null | undefined,
): IsanAgentTargetResolution {
  if (!target) {
    return {
      ok: false,
      error: describeUnresolvedIsanAgentTarget(
        selectedModelId,
        knownKeyProvider,
      ),
    };
  }
  return { ok: true, target };
}
