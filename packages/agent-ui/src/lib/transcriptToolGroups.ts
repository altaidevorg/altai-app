/**
 * Shared tool-part grouping for assistant transcript renders.
 *
 * Wave 4 / A6.3: collapse consecutive read/web/shell tool calls so hosts
 * (Desktop, VS Code) share the same run-burst rules without depending on the
 * Vercel AI SDK part object types.
 */

export type ToolLikePart = {
  type?: string;
  toolName?: string;
  toolCallId?: string;
  state?: string;
  input?: unknown;
  approval?: { id?: string };
};

export type TranscriptGroupKind = "reads" | "web" | "cmd";

export type TranscriptPartGroup<T = ToolLikePart> =
  | { kind: "single"; part: T; idx: number; key: string }
  | { kind: "reads"; parts: T[]; key: string }
  | { kind: "web"; parts: T[]; key: string }
  | { kind: "cmd"; parts: T[]; key: string };

const READ_GROUP_TOOLS = new Set(["read_file"]);
const WEB_GROUP_TOOLS = new Set([
  "web_search",
  "web_fetch",
  "arxiv_search",
  "arxiv_fetch",
  "hf_hub_file_fetch",
]);
const CMD_GROUP_TOOLS = new Set([
  "exec",
  "execution_run",
  "execution_run_background",
]);

function asToolLike(part: unknown): ToolLikePart {
  return (part ?? {}) as ToolLikePart;
}

/** Normalize AI SDK static tools + IsanAgent dynamic-tool envelopes. */
export function toolNameOf(part: ToolLikePart): string | null {
  const type = part.type ?? "";
  if (!type) return null;
  if (type === "dynamic-tool") {
    return part.toolName ?? null;
  }
  if (type.startsWith("tool-")) {
    return type.slice("tool-".length);
  }
  return null;
}

/**
 * Collapsible run kind for a tool part, or null for singles / approvals.
 * Approval cards always stay ungrouped so the user can act on them.
 */
export function groupKindFor(part: ToolLikePart): TranscriptGroupKind | null {
  const state = part.state ?? "";
  if (state === "approval-requested") return null;
  const name = toolNameOf(part);
  if (!name) return null;
  if (READ_GROUP_TOOLS.has(name)) return "reads";
  if (WEB_GROUP_TOOLS.has(name)) return "web";
  if (CMD_GROUP_TOOLS.has(name)) return "cmd";
  return null;
}

export function transcriptPartKey(part: ToolLikePart, idx: number): string {
  if (part.toolCallId) return part.toolCallId;
  const id = part.approval?.id;
  if (id) return id;
  return `i-${idx}`;
}

/**
 * Collapse consecutive groupable tools (≥2) into a run; leave singles alone.
 * Preserves the host part type `T` (e.g. AI SDK `UIMessagePart`).
 */
export function buildTranscriptPartGroups<T>(
  parts: readonly T[],
): TranscriptPartGroup<T>[] {
  const out: TranscriptPartGroup<T>[] = [];
  let run: {
    kind: TranscriptGroupKind;
    parts: T[];
    startIdx: number;
  } | null = null;

  const flushRun = () => {
    if (!run) return;
    if (run.parts.length >= 2) {
      out.push({
        kind: run.kind,
        parts: run.parts,
        key: `${run.kind}-${transcriptPartKey(asToolLike(run.parts[0]), run.startIdx)}`,
      });
    } else {
      run.parts.forEach((p, k) => {
        const idx = run!.startIdx + k;
        out.push({
          kind: "single",
          part: p,
          idx,
          key: transcriptPartKey(asToolLike(p), idx),
        });
      });
    }
    run = null;
  };

  parts.forEach((p, i) => {
    const kind = groupKindFor(asToolLike(p));
    if (kind) {
      if (run && run.kind === kind) {
        run.parts.push(p);
      } else {
        flushRun();
        run = { kind, parts: [p], startIdx: i };
      }
      return;
    }
    flushRun();
    out.push({
      kind: "single",
      part: p,
      idx: i,
      key: transcriptPartKey(asToolLike(p), i),
    });
  });
  flushRun();
  return out;
}

export function pathBasename(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i >= 0 ? path.slice(i + 1) : path;
}

export function readPathFromToolPart(part: ToolLikePart): string | null {
  const input = part.input;
  if (!input || typeof input !== "object" || Array.isArray(input)) {
    return null;
  }
  const path = (input as { path?: unknown }).path;
  return typeof path === "string" && path.length > 0 ? path : null;
}

export function uniqueReadPaths(parts: readonly ToolLikePart[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const p of parts) {
    const path = readPathFromToolPart(p);
    if (!path || seen.has(path)) continue;
    seen.add(path);
    out.push(path);
  }
  return out;
}

function toolInputString(input: unknown, key: string): string | null {
  if (!input || typeof input !== "object" || Array.isArray(input)) {
    return null;
  }
  const value = (input as Record<string, unknown>)[key];
  return typeof value === "string" ? value : null;
}

/** Collapsed-row summary fragment for a research tool call. */
export function webSummaryForToolPart(part: ToolLikePart): string | null {
  const name = toolNameOf(part);
  const input = part.input;
  if (!name || !input || typeof input !== "object") return null;
  const str = (k: string) => toolInputString(input, k);
  if (name === "web_search" || name === "arxiv_search") {
    const q = str("query");
    return q ? `"${q}"` : null;
  }
  if (name === "web_fetch") {
    const url = str("url");
    if (!url) return null;
    try {
      return new URL(url).hostname;
    } catch {
      return url;
    }
  }
  if (name === "arxiv_fetch") return str("arxiv_id");
  if (name === "hf_hub_file_fetch") {
    return str("repo_id") ?? str("repo") ?? str("path") ?? null;
  }
  return null;
}

/** First line of a shell/exec tool for the collapsed preview. */
export function cmdSummaryForToolPart(part: ToolLikePart): string | null {
  const name = toolNameOf(part);
  const input = part.input;
  if (!name || !input || typeof input !== "object") return null;
  const str = (k: string) => toolInputString(input, k);
  let raw: string | null = null;
  if (name === "exec") raw = str("description") ?? str("command");
  else if (name === "execution_run" || name === "execution_run_background") {
    raw = str("description") ?? str("code");
  }
  if (!raw) return null;
  const firstLine = raw.split("\n")[0]!.trim();
  return firstLine.length > 80 ? `${firstLine.slice(0, 79)}…` : firstLine;
}

export function uniqueSummaries(
  parts: readonly ToolLikePart[],
  summarize: (part: ToolLikePart) => string | null,
): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const p of parts) {
    const s = summarize(p);
    if (!s || seen.has(s)) continue;
    seen.add(s);
    out.push(s);
  }
  return out;
}

/**
 * Join a short head of previews, with `, +N more` when truncated.
 */
export function formatGroupPreview(
  items: readonly string[],
  options?: { separator?: string; max?: number },
): string | undefined {
  if (items.length === 0) return undefined;
  const max = options?.max ?? 3;
  const separator = options?.separator ?? ", ";
  const head = items.slice(0, max).join(separator);
  if (items.length <= max) return head;
  return `${head}, +${items.length - max} more`;
}
