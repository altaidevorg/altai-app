/**
 * Ports-first user-message edit form chrome (A6.52).
 * Hosts own submit/cancel handlers and busy state.
 */

import type { KeyboardEvent, ReactNode } from "react";

export type AiDisplayMessageEditFormProps = {
  value: string;
  disabled?: boolean;
  /** Label for the primary save button while idle. */
  saveLabel?: string;
  busyLabel?: string;
  cancelLabel?: string;
  ariaLabel?: string;
  rows?: number;
  onChange: (next: string) => void;
  onCancel: () => void;
  onSave: () => void;
  /** Optional trailing slot under action buttons. */
  footerNote?: ReactNode;
};

/**
 * Shared edit textarea + Cancel / Save & resend row.
 * Cmd/Ctrl+Enter saves when not composing; Escape cancels.
 */
export function AiDisplayMessageEditForm({
  value,
  disabled = false,
  saveLabel = "Save & resend",
  busyLabel = "Saving…",
  cancelLabel = "Cancel",
  ariaLabel = "Edit message",
  rows = 3,
  onChange,
  onCancel,
  onSave,
  footerNote,
}: AiDisplayMessageEditFormProps) {
  const canSave = Boolean(value.trim()) && !disabled;

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (
      event.key === "Enter" &&
      (event.metaKey || event.ctrlKey) &&
      !event.nativeEvent.isComposing
    ) {
      event.preventDefault();
      if (canSave) onSave();
    }
    if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
    }
  };

  return (
    <div className="altai-chat-edit">
      <textarea
        className="altai-chat-edit-input"
        value={value}
        rows={rows}
        aria-label={ariaLabel}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={onKeyDown}
      />
      <div className="altai-chat-edit-actions">
        <button
          type="button"
          className="altai-composer-stop"
          disabled={disabled}
          onClick={onCancel}
        >
          {cancelLabel}
        </button>
        <button
          type="button"
          className="altai-composer-submit"
          disabled={!canSave}
          onClick={onSave}
        >
          {disabled ? busyLabel : saveLabel}
        </button>
      </div>
      {footerNote}
    </div>
  );
}
