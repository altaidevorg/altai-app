import type { ReactNode, Ref, TextareaHTMLAttributes } from "react";
import { ComposerConfigRow } from "./ComposerConfigRow.js";
import { ComposerPrimaryRow } from "./ComposerPrimaryRow.js";
import { ComposerShell } from "./ComposerShell.js";
import { ComposerTextArea } from "./ComposerTextArea.js";
import { cn } from "../lib/cn.js";

export type AiComposerProps = {
  /** Text draft (controlled). Ignored when `draft` is provided. */
  value?: string;
  onChange?: (value: string) => void;
  /**
   * Full draft region replace (Desktop Popover+TextArea). When set, `value` /
   * `onChange` / built-in ComposerTextArea are not used.
   */
  draft?: ReactNode;
  /**
   * Attachment chips sit above the draft (Desktop `ComposerShell` attachments
   * slot). Omit when empty.
   */
  attachments?: ReactNode;
  /**
   * Optional suggestion pickers rendered adjacent to the default draft.
   * Ignored when `draft` is provided.
   */
  pickers?: ReactNode;
  /** Compact steer / queue strip under the draft when a run is active. */
  followup?: ReactNode;
  /** Model picker (shows config row when provided). */
  modelSlot?: ReactNode;
  /** Optional agent switcher (shows a 2-column config row). */
  agentSlot?: ReactNode;
  /** Primary-row tools cluster (attach, compact, checkpoints…). */
  tools: ReactNode;
  /** Permission mode control for the primary row. */
  permission?: ReactNode;
  /** Send / Stop control. */
  submit: ReactNode;
  busy?: boolean;
  disabled?: boolean;
  placeholder?: string;
  rows?: number;
  textareaRef?: Ref<HTMLTextAreaElement>;
  className?: string;
  inputClassName?: string;
  onKeyDown?: TextareaHTMLAttributes<HTMLTextAreaElement>["onKeyDown"];
  onKeyUp?: TextareaHTMLAttributes<HTMLTextAreaElement>["onKeyUp"];
  onClick?: TextareaHTMLAttributes<HTMLTextAreaElement>["onClick"];
  onSelect?: TextareaHTMLAttributes<HTMLTextAreaElement>["onSelect"];
};

/**
 * Shared AI composer frame used by Desktop (`AiInputBar`) and VS Code host.
 *
 * Presentational tree only: no Zustand, Tauri, or workspace I/O. Hosts inject
 * pickers, model/permission chrome, tools, and submit behavior via slots.
 * Layout order matches Desktop side-chat density (attachments → draft →
 * follow-up → config → primary tools/permission/submit).
 */
export function AiComposer({
  value = "",
  onChange,
  draft,
  attachments,
  pickers,
  followup,
  modelSlot,
  agentSlot,
  tools,
  permission,
  submit,
  busy = false,
  disabled = false,
  placeholder,
  rows = 2,
  textareaRef,
  className,
  inputClassName,
  onKeyDown,
  onKeyUp,
  onClick,
  onSelect,
}: AiComposerProps) {
  const showConfig = Boolean(modelSlot || agentSlot);

  return (
    <ComposerShell busy={busy} attachments={attachments} className={className}>
      {draft ?? (
        <div
          className={cn(
            "altai-ai-composer-input relative w-full min-w-0 px-3 pb-1 pt-2.5",
            inputClassName,
          )}
        >
          {pickers}
          <ComposerTextArea
            ref={textareaRef}
            value={value}
            onChange={(event) => {
              onChange?.(event.target.value);
            }}
            onKeyDown={onKeyDown}
            onKeyUp={onKeyUp}
            onClick={onClick}
            onSelect={onSelect}
            placeholder={placeholder}
            disabled={disabled}
            rows={rows}
          />
        </div>
      )}
      {followup}
      {showConfig ? (
        <ComposerConfigRow
          agentSlot={agentSlot}
          modelSlot={modelSlot ?? <span className="sr-only">Model</span>}
        />
      ) : null}
      <ComposerPrimaryRow tools={tools} permission={permission} submit={submit} />
    </ComposerShell>
  );
}
