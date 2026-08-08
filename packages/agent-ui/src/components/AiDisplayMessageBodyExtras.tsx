/**
 * Ports-first flat message body extras for diff/todos (A6.54).
 * Hosts own main text body; package owns optional inline chrome shells.
 */

import type { ReactNode } from "react";
import {
  TodoChecklist,
  type TodoItem,
} from "./TodoChecklist.js";
import { UnifiedDiffPreview } from "./UnifiedDiffPreview.js";

export type AiDisplayMessageBodyExtrasProps = {
  /** Inline review diff (both strings must be present to render). */
  originalText?: string;
  proposedText?: string;
  todos?: readonly TodoItem[];
  /** Optional main content above extras. */
  children?: ReactNode;
  denseTodos?: boolean;
};

/**
 * Renders optional UnifiedDiffPreview + TodoChecklist wrappers under
 * host-supplied children (markdown / user turn).
 */
export function AiDisplayMessageBodyExtras({
  originalText,
  proposedText,
  todos,
  children,
  denseTodos = true,
}: AiDisplayMessageBodyExtrasProps) {
  const showDiff =
    originalText !== undefined && proposedText !== undefined;
  const showTodos = Boolean(todos && todos.length > 0);

  return (
    <>
      {children}
      {showDiff ? (
        <div className="altai-chat-inline-diff">
          <UnifiedDiffPreview original={originalText} proposed={proposedText} />
        </div>
      ) : null}
      {showTodos ? (
        <div className="altai-chat-todos">
          <TodoChecklist items={[...(todos ?? [])]} dense={denseTodos} />
        </div>
      ) : null}
    </>
  );
}
