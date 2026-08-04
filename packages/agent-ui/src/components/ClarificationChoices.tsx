import {
  EditApprovalCard,
  type EditApprovalDiff,
} from "./EditApprovalCard.js";

export type ClarificationChoicesProps = {
  choices: string[] | null;
  editDiff: EditApprovalDiff | null;
  /** Host sends the selected reply (Desktop historically used sendMessage). */
  onRespond: (choice: string) => void;
};

/**
 * Pending clarification UI: either an edit-gate approval card or suggested
 * reply chips. Purely presentational; the host owns store + transport.
 */
export function ClarificationChoices({
  choices,
  editDiff,
  onRespond,
}: ClarificationChoicesProps) {
  // A file-edit approval (from the crate's edit gate) takes precedence over
  // the plain choice chips: it renders a richer diff-review card with
  // Approve / Deny actions. The reply still rides the clarification channel.
  if (editDiff) {
    return <EditApprovalCard diff={editDiff} onRespond={onRespond} />;
  }

  if (!choices || choices.length === 0) return null;

  return (
    <div
      role="group"
      aria-label="Suggested replies"
      className="flex shrink-0 flex-wrap gap-1.5 border-t border-border-subtle px-3 py-2"
    >
      <span aria-live="polite" className="sr-only">
        {choices.length} suggested{" "}
        {choices.length === 1 ? "reply" : "replies"} available
      </span>
      {choices.map((choice, i) => (
        <button
          key={`${i}-${choice}`}
          type="button"
          onClick={() => onRespond(choice)}
          className="rounded-md border border-border bg-muted px-3 py-1 text-[11px] font-medium text-foreground transition-colors hover:bg-accent"
        >
          {choice}
        </button>
      ))}
    </div>
  );
}
