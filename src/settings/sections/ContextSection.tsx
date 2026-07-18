import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  setCompactionAuto,
  setCompactionPrune,
  setCompactionPruneRecencyTokens,
  setCompactionTailTurns,
  setCompactionThresholdPercent,
  setCompactionThresholdTokens,
} from "@/modules/settings/store";
import { currentWorkspaceFolder } from "@/modules/workspace/folder";
import { native } from "@/modules/ai/lib/native";
import { runCompactNow } from "@/modules/ai/lib/slashCommands";
import { useChatStore } from "@/modules/ai/store/chatStore";
import { HugeiconsIcon } from "@hugeicons/react";
import { Archive02Icon } from "@hugeicons/core-free-icons";
import { useEffect, useState } from "react";
import { SectionHeader } from "../components/SectionHeader";
import { SettingRow } from "../components/SettingRow";

const isanagentignore_HELP = `# .isanagentignore — gitignore syntax. One file per workspace,
# at the root. Applies to ALTAI's own file access (editor search,
# explorer search, command-palette file search, and the file
# watcher). The agent's own tools (read/list/grep/glob) are
# enforced separately via altaidevorg/isanagent (Tier 2 PR).
secrets/**
*.env
!*.env.example
build/
dist/
`;

/**
 * Parse a number input value. Returns `null` for empty/unparseable input so
 * callers can distinguish "cleared" from "zero".
 */
function parseNumOrNull(raw: string): number | null {
  const trimmed = raw.trim();
  if (trimmed === "") return null;
  const n = Number(trimmed);
  return Number.isFinite(n) ? n : null;
}

export function ContextSection() {
  const compactionAuto = usePreferencesStore((s) => s.compactionAuto);
  const thresholdPercent = usePreferencesStore(
    (s) => s.compactionThresholdPercent,
  );
  const thresholdTokens = usePreferencesStore(
    (s) => s.compactionThresholdTokens,
  );
  const tailTurns = usePreferencesStore((s) => s.compactionTailTurns);
  const compactionPrune = usePreferencesStore((s) => s.compactionPrune);
  const pruneRecencyTokens = usePreferencesStore(
    (s) => s.compactionPruneRecencyTokens,
  );
  const hasActiveChat = useChatStore((s) => !!s.activeSessionId);

  return (
    <div className="flex flex-col gap-6">
      <SectionHeader
        title="Context"
        description="Control how ALTAI manages conversation context and which files it can access. Context-condensing settings take effect on the next message; .isanagentignore changes take effect immediately."
      />

      <SubsectionLabel>Context condensing</SubsectionLabel>
      <div className="flex flex-col gap-2">
        <SettingRow
          title="Auto-compaction"
          description="When the conversation approaches the model's context window, the runtime summarizes older history (keeping recent turns intact). Turning this off disables the automatic trigger; manual /compact still works."
        >
          <Switch
            checked={compactionAuto}
            onCheckedChange={(v) => void setCompactionAuto(v)}
          />
        </SettingRow>

        <SettingRow
          title="Threshold (% of context window)"
          description="Optional. When set, auto-compaction triggers at this share of the active model's context window — takes precedence over the absolute token value below. Empty = use the absolute token threshold."
        >
          <Input
            type="number"
            min={1}
            max={100}
            placeholder="—"
            className="h-7 w-24 rounded-md text-[12px]"
            value={thresholdPercent ?? ""}
            onChange={(e) => {
              const n = parseNumOrNull(e.target.value);
              void setCompactionThresholdPercent(
                n == null ? null : Math.min(100, Math.max(1, Math.round(n))),
              );
            }}
          />
        </SettingRow>

        <SettingRow
          title="Threshold (tokens)"
          description="Absolute token budget that triggers auto-compaction when no percent is set. Floored at 8k by the runtime so a typo can't wedge the loop."
        >
          <Input
            type="number"
            min={8000}
            step={1000}
            className="h-7 w-28 rounded-md text-[12px]"
            value={thresholdTokens}
            onChange={(e) => {
              const n = parseNumOrNull(e.target.value);
              if (n != null) void setCompactionThresholdTokens(Math.round(n));
            }}
          />
        </SettingRow>

        <SettingRow
          title="Recent turns to keep"
          description="Number of recent tool summaries the runtime preserves verbatim during compaction. Higher = more live context, less aggressive summarization."
        >
          <Input
            type="number"
            min={0}
            max={50}
            className="h-7 w-20 rounded-md text-[12px]"
            value={tailTurns}
            onChange={(e) => {
              const n = parseNumOrNull(e.target.value);
              if (n != null)
                void setCompactionTailTurns(
                  Math.min(50, Math.max(0, Math.round(n))),
                );
            }}
          />
        </SettingRow>

        <SettingRow
          title="Compact now"
          description="Manually trigger a between-turns compaction in the focused chat. Useful when auto-compaction is off or you want to reclaim context immediately."
        >
          <Button
            variant="outline"
            size="sm"
            disabled={!hasActiveChat}
            onClick={() => void runCompactNow()}
            className="h-7 shrink-0 gap-1.5 px-2.5 text-[11.5px]"
          >
            <HugeiconsIcon icon={Archive02Icon} size={12} strokeWidth={1.75} />
            Compact
          </Button>
        </SettingRow>

        <SettingRow
          title="Prune old tool results"
          description="Between turns, collapse completed tool outputs that fall outside a trailing recency window to '[Old tool result content cleared]'. Display/persistence only — keeps the transcript and on-disk history from ballooning. The model's own context is managed by the runtime's native compaction."
        >
          <Switch
            checked={compactionPrune}
            onCheckedChange={(v) => void setCompactionPrune(v)}
          />
        </SettingRow>

        <SettingRow
          title="Prune recency window (tokens)"
          description="Trailing token budget the prune pass keeps verbatim. Older completed tool outputs beyond this are collapsed. Estimated at ~4 chars/token."
        >
          <Input
            type="number"
            min={1000}
            step={1000}
            className="h-7 w-28 rounded-md text-[12px]"
            value={pruneRecencyTokens}
            onChange={(e) => {
              const n = parseNumOrNull(e.target.value);
              if (n != null)
                void setCompactionPruneRecencyTokens(
                  Math.max(1000, Math.round(n)),
                );
            }}
          />
        </SettingRow>
      </div>

      <SubsectionLabel>.isanagentignore</SubsectionLabel>
      <IsanagentignoreEditor />
    </div>
  );
}

function IsanagentignoreEditor() {
  const folder = currentWorkspaceFolder();
  const [content, setContent] = useState("");
  const [loaded, setLoaded] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoaded(false);
    setError(null);
    if (!folder) {
      setContent("");
      setLoaded(true);
      setDirty(false);
      return;
    }
    void native
      .getisanagentignore(folder)
      .then((existing) => {
        if (cancelled) return;
        setContent(existing ?? "");
        setDirty(false);
        setLoaded(true);
      })
      .catch((e) => {
        if (cancelled) return;
        setError(e instanceof Error ? e.message : String(e));
        setLoaded(true);
      });
    return () => {
      cancelled = true;
    };
  }, [folder]);

  const save = async () => {
    if (!folder) return;
    setSaving(true);
    setError(null);
    try {
      await native.setisanagentignore(content, folder);
      setDirty(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  if (!folder) {
    return (
      <div className="rounded-lg border border-border/60 bg-card/60 px-3 py-3 text-[11.5px] text-muted-foreground">
        Open a workspace folder to edit its <code>.isanagentignore</code>.
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2 rounded-lg border border-border/60 bg-card/60 px-3 py-3">
      <div className="flex items-center justify-between gap-2">
        <div className="flex min-w-0 flex-col gap-0.5">
          <span className="text-[12.5px] font-medium">
            <code>.isanagentignore</code>
          </span>
          <span className="text-[10.5px] leading-relaxed text-muted-foreground">
            Workspace-root ignore file (gitignore syntax). Filters ALTAI's file
            access: editor search, explorer search, command-palette file search,
            and the file watcher.
          </span>
        </div>
        <Button
          variant="outline"
          size="sm"
          disabled={!loaded || saving || !dirty}
          onClick={() => void save()}
          className="h-7 shrink-0 gap-1.5 px-2.5 text-[11.5px]"
        >
          <HugeiconsIcon icon={Archive02Icon} size={12} strokeWidth={1.75} />
          {saving ? "Saving…" : "Save"}
        </Button>
      </div>
      <textarea
        spellCheck={false}
        wrap="off"
        value={loaded ? content : ""}
        placeholder={isanagentignore_HELP}
        onChange={(e) => {
          setContent(e.target.value);
          setDirty(true);
        }}
        className={cn(
          "h-56 w-full resize-y rounded-md border border-border/60 bg-background/60 p-2 font-mono text-[11.5px] leading-relaxed outline-none",
          "focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/30",
        )}
      />
      <div className="flex flex-col gap-1 text-[10.5px] leading-relaxed text-muted-foreground">
        <span>
          <strong>Scope:</strong> editor search, explorer search,
          command-palette file search, and the file watcher. The agent's own
          tools (read/list/grep/glob) are enforced separately via the
          <code> altaidevorg/isanagent</code> crate (Tier 2 PR — tracked
          separately).
        </span>
        <span>
          <strong>Syntax:</strong> one pattern per line; <code>*</code> and
          <code> **</code> globs; trailing <code>/</code> matches directories;
          <code>!</code> negates; <code>#</code> starts a comment.
        </span>
      </div>
      {error ? (
        <span className="text-[10.5px] text-destructive">{error}</span>
      ) : null}
    </div>
  );
}

function SubsectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="-mb-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
      {children}
    </h3>
  );
}
