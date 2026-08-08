/**
 * Opaque message / request identifiers for Webview envelopes (A6.129).
 */

import { createSecureId } from "./secureRandom.js";

export function createMessageId(prefix = "msg"): string {
  return createSecureId(prefix);
}
