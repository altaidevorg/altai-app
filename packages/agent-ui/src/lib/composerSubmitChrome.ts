/**
 * Desktop-aligned composer Send / Stop chrome policy.
 * Presentational hosts only — no React / transport.
 */

export type ComposerSubmitChromeMode = "stop" | "send";

/**
 * Desktop shows Stop while a run is active/busy, otherwise the Send control.
 * Never both at once.
 */
export function composerSubmitChromeMode(input: {
  busy: boolean;
  hasActiveRun: boolean;
  isCancelling?: boolean;
}): ComposerSubmitChromeMode {
  if (input.busy || input.hasActiveRun || input.isCancelling) {
    return "stop";
  }
  return "send";
}

export function canEnableComposerSend(input: {
  busy: boolean;
  hasPrompt: boolean;
  canStartRun: boolean;
  hasActiveRun: boolean;
  canSteer: boolean;
  canQueue: boolean;
}): boolean {
  if (input.busy) {
    return false;
  }
  if (!input.hasPrompt) {
    return false;
  }
  if (input.canStartRun) {
    return true;
  }
  return Boolean(input.hasActiveRun && (input.canSteer || input.canQueue));
}

export function canEnableComposerStop(input: {
  hasActiveRun: boolean;
  busy: boolean;
  isCancelling?: boolean;
}): boolean {
  if (input.isCancelling) {
    return false;
  }
  return input.hasActiveRun || input.busy;
}
