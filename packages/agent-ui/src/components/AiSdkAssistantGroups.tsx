/**
 * Ports-first AI-SDK assistant part-group renderer (A6.41).
 * Hosts inject part cards (text / reason / tool) + path open; no Tauri/stores.
 */

import { useMemo, type ReactNode } from "react";
import {
  cmdSummaryForToolPart,
  formatGroupPreview,
  pathBasename,
  transcriptPartKey,
  uniqueReadPaths,
  uniqueSummaries,
  webSummaryForToolPart,
  type ToolLikePart,
  type TranscriptPartGroup,
} from "../lib/transcriptToolGroups.js";
import { isStandaloneReadToolPart } from "../lib/chatSdkAssistantChrome.js";
import { TranscriptToolGroup } from "./TranscriptToolGroup.js";
import { TranscriptReadPaths } from "./TranscriptReadPaths.js";
import { TranscriptReadRow } from "./TranscriptReadRow.js";
import { cn } from "../lib/cn.js";

export type AiSdkAssistantGroupsProps<T = ToolLikePart> = {
  messageId: string;
  groups: readonly TranscriptPartGroup<T>[];
  /** When true, the last text part of the message is live. */
  streaming: boolean;
  lastTextPartIdx: number;
  onApproval: (id: string, approved: boolean) => void;
  /** Host opens a workspace path from read groups. */
  onOpenPath: (path: string) => void;
  /**
   * Render one ungrouped or in-group part (text, reasoning, tool, approval).
   * Hosts supply MessageResponse / Tool / host-specific chrome.
   */
  renderPart: (input: {
    part: T;
    streaming: boolean;
    onApproval: (id: string, approved: boolean) => void;
  }) => ReactNode;
  /** Optional enter animation wrapper (Desktop uses motion). Default: div. */
  wrapPart?: (node: ReactNode, key: string) => ReactNode;
  /** Icons for collapsible groups (Hugeicons from host). */
  icons: {
    file: ReactNode;
    web: ReactNode;
    terminal: ReactNode;
  };
  className?: string;
};

function defaultWrap(node: ReactNode, key: string): ReactNode {
  return <div key={key}>{node}</div>;
}

/**
 * Map `buildTranscriptPartGroups` output to tool bursts + host part cards.
 */
export function AiSdkAssistantGroups<T = ToolLikePart>({
  messageId,
  groups,
  streaming,
  lastTextPartIdx,
  onApproval,
  onOpenPath,
  renderPart,
  wrapPart = defaultWrap,
  icons,
  className,
}: AiSdkAssistantGroupsProps<T>) {
  return (
    <div className={cn("flex min-w-0 flex-col gap-3", className)}>
      {groups.map((g) => {
        const key = `${messageId}-${g.key}`;
        if (g.kind === "reads") {
          return wrapPart(
            <ReadBurst
              parts={g.parts as ToolLikePart[]}
              icon={icons.file}
              onOpenPath={onOpenPath}
            />,
            key,
          );
        }
        if (g.kind === "web") {
          return wrapPart(
            <NamedBurst
              label="Web"
              countLabel={`${g.parts.length} call${g.parts.length === 1 ? "" : "s"}`}
              preview={formatGroupPreview(
                uniqueSummaries(
                  g.parts as ToolLikePart[],
                  webSummaryForToolPart,
                ),
              )}
              icon={icons.web}
              parts={g.parts}
              onApproval={onApproval}
              renderPart={renderPart}
            />,
            key,
          );
        }
        if (g.kind === "cmd") {
          return wrapPart(
            <NamedBurst
              label="Ran"
              countLabel={`${g.parts.length} command${g.parts.length === 1 ? "" : "s"}`}
              preview={formatGroupPreview(
                uniqueSummaries(
                  g.parts as ToolLikePart[],
                  cmdSummaryForToolPart,
                ),
                { separator: " · " },
              )}
              previewMono
              icon={icons.terminal}
              parts={g.parts}
              onApproval={onApproval}
              renderPart={renderPart}
            />,
            key,
          );
        }

        const part = g.part;
        const asTool = part as ToolLikePart;
        if (isStandaloneReadToolPart(asTool)) {
          return wrapPart(<TranscriptReadRow part={asTool} />, key);
        }
        return wrapPart(
          renderPart({
            part,
            streaming: streaming && g.idx === lastTextPartIdx,
            onApproval,
          }),
          key,
        );
      })}
    </div>
  );
}

function ReadBurst({
  parts,
  icon,
  onOpenPath,
}: {
  parts: ToolLikePart[];
  icon: ReactNode;
  onOpenPath: (path: string) => void;
}) {
  const paths = useMemo(() => uniqueReadPaths(parts), [parts]);
  const count = paths.length || parts.length;
  return (
    <TranscriptToolGroup
      label="Read"
      countLabel={`${count} file${count === 1 ? "" : "s"}`}
      preview={
        paths.length > 0
          ? formatGroupPreview(paths.map((p) => pathBasename(p)))
          : undefined
      }
      previewMono
      icon={icon}
    >
      <TranscriptReadPaths paths={paths} onOpen={onOpenPath} />
    </TranscriptToolGroup>
  );
}

function NamedBurst<T>({
  label,
  countLabel,
  preview,
  previewMono,
  icon,
  parts,
  onApproval,
  renderPart,
}: {
  label: string;
  countLabel: string;
  preview?: string;
  previewMono?: boolean;
  icon: ReactNode;
  parts: readonly T[];
  onApproval: (id: string, approved: boolean) => void;
  renderPart: AiSdkAssistantGroupsProps<T>["renderPart"];
}) {
  return (
    <TranscriptToolGroup
      label={label}
      countLabel={countLabel}
      preview={preview}
      previewMono={previewMono}
      icon={icon}
    >
      <div className="flex flex-col gap-1 px-2 py-1.5">
        {parts.map((p, i) => (
          <div key={transcriptPartKey(p as ToolLikePart, i)}>
            {renderPart({
              part: p,
              streaming: false,
              onApproval,
            })}
          </div>
        ))}
      </div>
    </TranscriptToolGroup>
  );
}
