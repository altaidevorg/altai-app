import { cn } from "../lib/cn.js";

export type UnifiedDiffPreviewProps = {
  original: string;
  proposed: string;
};

type DiffLine = { kind: "add" | "del" | "ctx"; text: string };

/**
 * Coarse line-level unified diff preview. Renders removed lines in
 * destructive tone and added lines in success tone, capped at 80 lines.
 * Purely presentational; the host supplies original and proposed text.
 */
export function UnifiedDiffPreview({
  original,
  proposed,
}: UnifiedDiffPreviewProps) {
  const a = original.split("\n");
  const b = proposed.split("\n");
  const setA = new Set(a);
  const setB = new Set(b);

  const lines: DiffLine[] = [];
  for (const l of a) if (!setB.has(l)) lines.push({ kind: "del", text: l });
  for (const l of b) if (!setA.has(l)) lines.push({ kind: "add", text: l });

  if (lines.length === 0) {
    return (
      <div className="text-[11px] italic text-muted-foreground">
        no line-level changes
      </div>
    );
  }

  const MAX = 80;
  const shown = lines.slice(0, MAX);
  const rest = lines.length - shown.length;

  return (
    <div className="overflow-hidden rounded border border-border/40 font-mono text-[11px] leading-relaxed">
      <div className="max-h-72 overflow-auto">
        {shown.map((l, i) => (
          <div
            key={i}
            className={cn(
              "flex whitespace-pre",
              l.kind === "add"
                ? "bg-success/10 text-success"
                : "bg-destructive/10 text-destructive",
            )}
          >
            <span className="w-4 shrink-0 select-none px-1 text-center opacity-70">
              {l.kind === "add" ? "+" : "-"}
            </span>
            <span className="min-w-0 flex-1 overflow-x-auto pr-2">
              {l.text || " "}
            </span>
          </div>
        ))}
        {rest > 0 ? (
          <div className="px-2 py-1 text-[10px] italic text-muted-foreground">
            … {rest} more changes
          </div>
        ) : null}
      </div>
    </div>
  );
}
