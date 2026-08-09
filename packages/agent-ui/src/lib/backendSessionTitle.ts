/**
 * Pure backend session title normalize (A6.181).
 * Recovered rows often omit or blank titles; UI uses "New chat".
 */

/** Default untitled-chat title used across session surfaces. */
export const DEFAULT_SESSION_TITLE = "New chat";

/** Backend preview title for recovered sessions; blank → New chat. */
export function backendSessionTitle(title: string | null | undefined): string {
  return title?.trim() || DEFAULT_SESSION_TITLE;
}

/** UI session-row title when stored title is empty. */
export function displaySessionTitle(title: string | null | undefined): string {
  return title?.trim() || DEFAULT_SESSION_TITLE;
}
