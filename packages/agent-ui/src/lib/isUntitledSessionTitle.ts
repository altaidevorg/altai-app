/**
 * Pure untitled session title check (A6.188).
 */

import { DEFAULT_SESSION_TITLE } from "./backendSessionTitle.js";

/** True when a session title is missing or the default untitled label. */
export function isUntitledSessionTitle(title: string | null | undefined): boolean {
  return !title || title === DEFAULT_SESSION_TITLE;
}
