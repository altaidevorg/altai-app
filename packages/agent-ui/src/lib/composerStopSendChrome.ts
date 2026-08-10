/**
 * Pure Composer Stop/Send control labels (A6.257).
 */

export function composerStopControlLabel(isCancelling: boolean): string {
  return isCancelling ? "Stopping" : "Stop";
}

export function composerStopAriaLabel(isCancelling: boolean): string {
  return isCancelling ? "Cancelling" : "Stop";
}

export const COMPOSER_SEND_TOOLTIP = "Send · Enter";
export const COMPOSER_SEND_ARIA_LABEL = "Send";
