import {
  Attachment01Icon,
  Cancel01Icon,
  CodeIcon,
  HashtagIcon,
  TerminalIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ReactNode } from "react";

export type ComposerAttachCommand = {
  name: string;
  label: string;
  icon: ReactNode;
};

export type ComposerAttachSnippet = {
  id: string;
  handle: string;
  description?: string;
};

export type ComposerAttachFile = {
  id: string;
  name: string;
  kind: "image" | "pdf" | "text" | "selection" | "terminal" | "diff" | "folder";
  url?: string;
  text?: string;
  source?: "editor" | "terminal";
};

export type ComposerAttachChipsProps = {
  files: ComposerAttachFile[];
  onRemoveFile: (id: string) => void;
  snippets: ComposerAttachSnippet[];
  onRemoveSnippet: (id: string) => void;
  commands: ComposerAttachCommand[];
  onRemoveCommand: (name: string) => void;
  contextTokenEstimate?: number;
};

export function selectionLineCount(text: string): number {
  if (!text) return 0;
  const trimmed = text.replace(/\n+$/, "");
  if (!trimmed) return 0;
  return trimmed.split("\n").length;
}

export function fileExtensionLabel(name: string): string {
  const i = name.lastIndexOf(".");
  return i === -1 ? "FILE" : name.slice(i + 1).toUpperCase();
}

function formatTokenEstimate(n: number): string {
  return n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n);
}

/**
 * Removable composer attachment chips (commands, snippets, files). Presentational
 * only — no motion library; the host owns attachment state.
 */
export function ComposerAttachChips({
  files,
  onRemoveFile,
  snippets,
  onRemoveSnippet,
  commands,
  onRemoveCommand,
  contextTokenEstimate = 0,
}: ComposerAttachChipsProps) {
  if (files.length === 0 && snippets.length === 0 && commands.length === 0) {
    return null;
  }

  return (
    <div className="altai-composer-attach-chips flex flex-wrap gap-1">
      {commands.map((cmd) => (
        <div
          key={`cmd-${cmd.name}`}
          className="group flex items-center gap-1 rounded-md border border-border-subtle bg-card px-1.5 py-0.5 text-[11px]"
          title={cmd.label}
        >
          {cmd.icon}
          <span className="font-medium">#{cmd.name}</span>
          <button
            type="button"
            onClick={() => onRemoveCommand(cmd.name)}
            className="ml-0.5 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
            aria-label="Remove command"
          >
            <HugeiconsIcon icon={Cancel01Icon} size={10} strokeWidth={2} />
          </button>
        </div>
      ))}
      {snippets.map((s) => (
        <div
          key={`snip-${s.id}`}
          className="group flex items-center gap-1 rounded-md border border-border-subtle bg-card px-1.5 py-0.5 text-[11px] text-foreground"
          title={s.description || s.handle}
        >
          <HugeiconsIcon
            icon={HashtagIcon}
            size={11}
            strokeWidth={2}
            className="opacity-80"
          />
          <span className="font-medium">{s.handle}</span>
          <button
            type="button"
            onClick={() => onRemoveSnippet(s.id)}
            className="ml-0.5 opacity-0 transition-opacity group-hover:opacity-100"
            aria-label="Remove snippet"
          >
            <HugeiconsIcon icon={Cancel01Icon} size={10} strokeWidth={2} />
          </button>
        </div>
      ))}
      {files.map((f) => (
        <div
          key={f.id}
          className="group flex items-center gap-1 rounded-md border border-border-subtle bg-card px-1.5 py-0.5 text-[11px]"
        >
          {fileLeading(f)}
          <span className="max-w-35 truncate">
            {f.name}
            {f.kind === "selection" && f.text ? (
              <span className="ml-1 text-muted-foreground">
                · {selectionLineCount(f.text)}L
              </span>
            ) : null}
            {(f.kind === "terminal" ||
              f.kind === "diff" ||
              f.kind === "folder") &&
            f.text ? (
              <span className="ml-1 text-muted-foreground">
                · {selectionLineCount(f.text)}L
              </span>
            ) : null}
          </span>
          <button
            type="button"
            onClick={() => onRemoveFile(f.id)}
            className="ml-0.5 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
            aria-label="Remove"
          >
            <HugeiconsIcon icon={Cancel01Icon} size={10} strokeWidth={2} />
          </button>
        </div>
      ))}
      {contextTokenEstimate > 0 ? (
        <span
          className="self-center px-1 text-[10px] tabular-nums text-muted-foreground"
          title="Approximate attached context tokens"
        >
          ~{formatTokenEstimate(contextTokenEstimate)} tokens
        </span>
      ) : null}
    </div>
  );
}

function fileLeading(f: ComposerAttachFile): ReactNode {
  if (f.kind === "image" && f.url) {
    return <img src={f.url} alt="" className="size-4 rounded object-cover" />;
  }
  if (f.kind === "selection") {
    return (
      <HugeiconsIcon
        icon={f.source === "editor" ? CodeIcon : TerminalIcon}
        size={11}
        strokeWidth={1.75}
        className="text-muted-foreground"
      />
    );
  }
  if (f.kind === "terminal") {
    return (
      <HugeiconsIcon
        icon={TerminalIcon}
        size={11}
        strokeWidth={1.75}
        className="text-info"
      />
    );
  }
  if (f.kind === "diff") {
    return (
      <HugeiconsIcon
        icon={CodeIcon}
        size={11}
        strokeWidth={1.75}
        className="text-warning"
      />
    );
  }
  if (f.kind === "folder") {
    return (
      <HugeiconsIcon
        icon={Attachment01Icon}
        size={11}
        strokeWidth={1.75}
        className="text-primary"
      />
    );
  }
  return (
    <span className="font-mono text-[10px] text-muted-foreground">
      {fileExtensionLabel(f.name)}
    </span>
  );
}
