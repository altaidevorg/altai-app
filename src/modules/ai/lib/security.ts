/**
 * Path-safety guards for AI tool calls.
 *
 * Shared pure implementation lives in `@altai/agent-ui` (A6.158).
 * This module is a thin host re-export so Desktop import paths stay stable.
 */

export type { SafetyResult } from "@altai/agent-ui";
export {
  checkReadable,
  checkWritable,
  checkReadableCanonical,
  checkWritableCanonical,
  checkShellCommand,
} from "@altai/agent-ui";
