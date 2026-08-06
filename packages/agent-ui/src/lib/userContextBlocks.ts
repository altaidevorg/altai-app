import type { ContextChip } from "../components/ContextChips.js";

function countLines(s: string): number {
  if (!s) return 0;
  const trimmed = s.replace(/\n+$/, "");
  if (!trimmed) return 0;
  return trimmed.split("\n").length;
}

/**
 * Strip host-injected user context XML blocks from message text, returning
 * display text plus typed chips for `ContextChips`.
 *
 * Wave 4 / A6.1: shared between Desktop AiChat and any host that embeds the
 * same selection/file/diff markup.
 *
 * Regexes are built per call so global `lastIndex` state cannot leak across
 * invokes (which would reorder or drop chips).
 */
export function stripUserContextBlocks(text: string): {
  text: string;
  chips: ContextChip[];
} {
  const chips: ContextChip[] = [];
  let out = text;
  out = out.replace(
    /<selection\s+source="(terminal|editor)">\n?([\s\S]*?)\n?<\/selection>/g,
    (_m, source: string, body: string) => {
      chips.push({
        kind: "selection",
        source: source === "editor" ? "editor" : "terminal",
        lines: countLines(body),
      });
      return "";
    },
  );
  out = out.replace(
    /<file\s+name="([^"]+)"[^>]*>\n?([\s\S]*?)\n?<\/file>/g,
    (_m, name: string, body: string) => {
      chips.push({ kind: "file", name, lines: countLines(body) });
      return "";
    },
  );
  out = out.replace(
    /<terminal-context(?:\s+name="([^"]+)")?>\n?([\s\S]*?)\n?<\/terminal-context>/g,
    (_m, name: string | undefined, body: string) => {
      chips.push({
        kind: "terminal",
        name: name || "Active terminal",
        lines: countLines(body),
      });
      return "";
    },
  );
  out = out.replace(
    /<git-diff(?:\s+name="([^"]+)")?>\n?([\s\S]*?)\n?<\/git-diff>/g,
    (_m, name: string | undefined, body: string) => {
      chips.push({
        kind: "diff",
        name: name || "Working tree diff",
        lines: countLines(body),
      });
      return "";
    },
  );
  out = out.replace(
    /<folder\s+name="([^"]+)">\n?([\s\S]*?)\n?<\/folder>/g,
    (_m, name: string, body: string) => {
      chips.push({ kind: "folder", name, lines: countLines(body) });
      return "";
    },
  );
  out = out.replace(
    /<snippet\s+name="([^"]+)">\n?[\s\S]*?\n?<\/snippet>/g,
    (_m, name: string) => {
      chips.push({ kind: "snippet", name });
      return "";
    },
  );
  return { text: out.trim(), chips };
}
