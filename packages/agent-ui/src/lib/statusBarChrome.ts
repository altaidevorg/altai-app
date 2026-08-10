/**
 * Pure AI status-bar control titles (A6.252).
 */

/** Title for the Hide/Show AI agent control. Host formats the shortcut. */
export function aiAgentToggleTitle(
  active: boolean,
  shortcutLabel: string,
): string {
  return `${active ? "Hide" : "Show"} AI agent  ${shortcutLabel}`;
}

export type VoiceControlState = {
  hasKey: boolean;
  recording: boolean;
  transcribing: boolean;
};

/** Tooltip for the voice input status control. */
export function voiceInputControlTitle(state: VoiceControlState): string {
  if (!state.hasKey) return "Voice needs an OpenAI key";
  if (state.recording) return "Stop & transcribe";
  if (state.transcribing) return "Transcribing…";
  return "Voice input";
}

/** Whether the voice control should be disabled. */
export function voiceInputControlDisabled(
  isBusy: boolean,
  state: VoiceControlState,
): boolean {
  return isBusy || state.transcribing || !state.hasKey;
}
