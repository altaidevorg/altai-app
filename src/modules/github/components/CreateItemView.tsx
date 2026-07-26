import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import {
  buildIssueBody,
  createIssue,
  createPull,
  type GHItem,
  type GHLabel,
  type ItemKind,
  listBranches,
  listLabels,
  type RepoSlug,
} from "@/modules/github/lib/items";
import { assignGitHubItem } from "@/modules/github/store/assignmentsStore";
import { useChatStore } from "@/modules/ai/store/chatStore";
import { useAgentsStore } from "@/modules/ai/store/agentsStore";
import { usePreferencesStore } from "@/modules/settings/preferences";
import { ArrowLeft01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useState } from "react";
import {
  AgentRunOptionsFields,
  type AgentRunOptions,
} from "./AgentRunOptionsFields";

type Props = {
  slug: RepoSlug;
  kind: ItemKind;
  onBack: () => void;
  onCreated: (item: GHItem) => void;
};

function pickDefaultBase(branches: string[]): string {
  return (
    branches.find((b) => b === "main" || b === "master") ?? branches[0] ?? ""
  );
}

export function CreateItemView({ slug, kind, onBack, onCreated }: Props) {
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [acceptanceCriteria, setAcceptanceCriteria] = useState("");
  const [assignAfterCreate, setAssignAfterCreate] = useState(false);
  const [isolateWorktree, setIsolateWorktree] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [createdItem, setCreatedItem] = useState<GHItem | null>(null);
  const activeAgentId = useAgentsStore((s) => s.activeId);
  const selectedModelId = useChatStore((s) => s.selectedModelId);
  const defaultPermissionMode = usePreferencesStore((s) => s.permissionMode);
  const bypassEnabled = usePreferencesStore((s) => s.bypassPermissionsEnabled);
  const [runOptions, setRunOptions] = useState<AgentRunOptions>(() => ({
    agentId: activeAgentId,
    modelId: selectedModelId,
    permissionMode:
      defaultPermissionMode === "bypass" && !bypassEnabled
        ? "ask"
        : defaultPermissionMode,
  }));

  // Issue: label options. PR: branch options.
  const [labels, setLabels] = useState<GHLabel[]>([]);
  const [selectedLabels, setSelectedLabels] = useState<Set<string>>(new Set());
  const [branches, setBranches] = useState<string[]>([]);
  const [baseRef, setBaseRef] = useState("");
  const [headRef, setHeadRef] = useState("");

  useEffect(() => {
    let alive = true;
    if (kind === "issues") {
      listLabels(slug)
        .then((l) => alive && setLabels(l))
        .catch(() => {});
    } else {
      listBranches(slug)
        .then((b) => {
          if (!alive) return;
          setBranches(b);
          const base = pickDefaultBase(b);
          setBaseRef(base);
          setHeadRef(b.find((x) => x !== base) ?? base);
        })
        .catch(() => {});
    }
    return () => {
      alive = false;
    };
  }, [slug, kind]);

  const valid =
    title.trim().length > 0 &&
    (kind === "issues" || (baseRef && headRef && baseRef !== headRef));

  const submit = async () => {
    if (!valid || busy || createdItem) return;
    setBusy(true);
    setError(null);
    try {
      const issueBody = buildIssueBody(body, acceptanceCriteria);
      const item =
        kind === "issues"
          ? await createIssue(slug, {
              title: title.trim(),
              body: issueBody,
              labels: [...selectedLabels],
            })
          : await createPull(slug, {
              title: title.trim(),
              body: body.trim(),
              base: baseRef,
              head: headRef,
            });
      setCreatedItem(item);
      if (kind === "issues" && assignAfterCreate) {
        try {
          await assignGitHubItem({
            kind: "issue",
            slug,
            number: item.number,
            title: item.title,
            body: issueBody,
            url: item.html_url,
            runConfig: runOptions,
            isolate: isolateWorktree,
          });
        } catch (cause) {
          setError(
            `Issue #${item.number} was created, but the agent couldn't start: ${
              cause instanceof Error ? cause.message : String(cause)
            }`,
          );
          return;
        }
      }
      onCreated(item);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const toggleLabel = (name: string) =>
    setSelectedLabels((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });

  const selectClass =
    "h-8 rounded-lg border border-border/60 bg-background/60 px-2 text-[12px] text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring";

  return (
    <div className="mx-auto flex h-full w-full max-w-2xl flex-col gap-3 overflow-y-auto px-4 py-3">
      <button
        type="button"
        onClick={onBack}
        className="flex w-fit items-center gap-1 text-[11.5px] text-muted-foreground transition-colors hover:text-foreground"
      >
        <HugeiconsIcon icon={ArrowLeft01Icon} size={13} strokeWidth={2} />
        Back to list
      </button>

      <h2 className="text-[14px] font-semibold text-foreground">
        {kind === "pulls" ? "New pull request" : "New issue"}
      </h2>

      {kind === "pulls" ? (
        <div className="flex items-center gap-2">
          <select
            value={baseRef}
            onChange={(e) => setBaseRef(e.target.value)}
            aria-label="Base branch"
            className={cn(selectClass, "min-w-0 flex-1")}
          >
            {branches.map((b) => (
              <option key={b} value={b}>
                base: {b}
              </option>
            ))}
          </select>
          <span className="text-muted-foreground">←</span>
          <select
            value={headRef}
            onChange={(e) => setHeadRef(e.target.value)}
            aria-label="Compare branch"
            className={cn(selectClass, "min-w-0 flex-1")}
          >
            {branches.map((b) => (
              <option key={b} value={b}>
                compare: {b}
              </option>
            ))}
          </select>
        </div>
      ) : null}

      <Input
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        placeholder="Title"
        aria-label="Title"
        className="text-[13px]"
      />

      <Textarea
        value={body}
        onChange={(e) => setBody(e.target.value)}
        placeholder="Leave a description… (Markdown supported)"
        aria-label="Description"
        rows={8}
        className="resize-none text-[12px]"
      />

      {kind === "issues" ? (
        <div className="flex flex-col gap-2">
          <div>
            <label
              htmlFor="issue-acceptance-criteria"
              className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground/70"
            >
              Acceptance criteria
            </label>
            <Textarea
              id="issue-acceptance-criteria"
              value={acceptanceCriteria}
              onChange={(event) => setAcceptanceCriteria(event.target.value)}
              placeholder={"One requirement per line\nRelevant tests pass\nExisting behavior remains unchanged"}
              rows={4}
              className="mt-1.5 resize-none text-[12px]"
            />
            <p className="mt-1 text-[10.5px] text-muted-foreground/60">
              Saved as a GitHub checklist so progress stays visible outside ALTAI.
            </p>
          </div>

          <button
            type="button"
            aria-pressed={assignAfterCreate}
            onClick={() => setAssignAfterCreate((value) => !value)}
            className={cn(
              "flex items-center gap-2 rounded-xl border px-3 py-2 text-left transition-colors",
              assignAfterCreate
                ? "border-sky-500/35 bg-sky-500/8"
                : "border-border/60 bg-background/30 hover:bg-muted/30",
            )}
          >
            <span
              className={cn(
                "flex size-4 items-center justify-center rounded border text-[10px]",
                assignAfterCreate
                  ? "border-sky-500 bg-sky-500 text-white"
                  : "border-border",
              )}
            >
              {assignAfterCreate ? "✓" : ""}
            </span>
            <span>
              <span className="block text-[11.5px] font-medium text-foreground">
                Create and assign to agent
              </span>
              <span className="block text-[10.5px] text-muted-foreground">
                Starts an independent background run after GitHub creates the issue.
              </span>
            </span>
          </button>

          {assignAfterCreate ? (
            <>
              <AgentRunOptionsFields
                value={runOptions}
                onChange={setRunOptions}
                disabled={busy}
              />
              <button
                type="button"
                aria-pressed={isolateWorktree}
                onClick={() => setIsolateWorktree((value) => !value)}
                className={cn(
                  "flex items-center gap-2 rounded-xl border px-3 py-2 text-left transition-colors",
                  isolateWorktree
                    ? "border-emerald-500/35 bg-emerald-500/8"
                    : "border-amber-500/35 bg-amber-500/8",
                )}
              >
                <span
                  className={cn(
                    "flex size-4 items-center justify-center rounded border text-[10px]",
                    isolateWorktree
                      ? "border-emerald-500 bg-emerald-500 text-white"
                      : "border-amber-500",
                  )}
                >
                  {isolateWorktree ? "✓" : ""}
                </span>
                <span className="text-[11px] text-foreground">
                  Isolate changes in a dedicated git worktree
                </span>
              </button>
            </>
          ) : null}
        </div>
      ) : null}

      {kind === "issues" && labels.length > 0 ? (
        <div className="flex flex-col gap-1.5">
          <p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground/60">
            Labels
          </p>
          <div className="flex flex-wrap gap-1.5">
            {labels.map((l) => {
              const on = selectedLabels.has(l.name);
              return (
                <button
                  key={l.id}
                  type="button"
                  onClick={() => toggleLabel(l.name)}
                  className={cn(
                    "flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[10.5px] font-medium transition-colors",
                    on
                      ? "border-transparent text-foreground"
                      : "border-border/60 text-muted-foreground hover:text-foreground",
                  )}
                  style={
                    on ? { backgroundColor: `#${l.color}33` } : undefined
                  }
                >
                  <span
                    className="size-2 rounded-full"
                    style={{ backgroundColor: `#${l.color}` }}
                  />
                  {l.name}
                </button>
              );
            })}
          </div>
        </div>
      ) : null}

      {error ? <p className="text-[11.5px] text-destructive">{error}</p> : null}

      <div className="flex items-center gap-2">
        {createdItem ? (
          <Button
            size="sm"
            variant="outline"
            className="ml-auto h-8 text-[12px]"
            onClick={() => onCreated(createdItem)}
          >
            Open issue #{createdItem.number}
          </Button>
        ) : null}
        <Button
          size="sm"
          className={cn("h-8 text-[12px]", !createdItem && "ml-auto")}
          onClick={() => void submit()}
          disabled={!valid || busy || !!createdItem}
        >
          {busy ? (
            <Spinner className="size-3.5" />
          ) : kind === "pulls" ? (
            "Create pull request"
          ) : (
            "Create issue"
          )}
        </Button>
      </div>
    </div>
  );
}
