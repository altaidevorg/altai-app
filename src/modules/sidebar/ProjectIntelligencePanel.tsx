import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  PROJECT_INSTRUCTIONS_FILE,
  projectInstructionsPath,
} from "@/modules/ai/lib/projectInstructions";
import { native } from "@/modules/ai/lib/native";
import {
  ArrowDown01Icon,
  CheckmarkCircle02Icon,
  InformationCircleIcon,
  Refresh01Icon,
  Tick02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";

const STARTER = `# ALTAI project instructions

## Project
- What this application does:
- Important architecture decisions:

## Development
- Install command:
- Test command:
- Lint/type-check command:

## Conventions
- Naming and formatting:
- Files or areas that need extra care:
- Things the agent must not change:
`;

type LoadState = "loading" | "ready" | "missing" | "unavailable" | "error";

/**
 * Collapsible panel that exposes the human-written project contract
 * (project instructions file) directly inside the Operations
 * sidebar, so project-scoped context is kept alongside project work.
 */
export function ProjectIntelligencePanel({
  defaultOpen = false,
}: {
  defaultOpen?: boolean;
}) {
  const [open, setOpen] = useState(defaultOpen);
  const [workspace, setWorkspace] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [state, setState] = useState<LoadState>("loading");
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const load = async () => {
    setNotice(null);
    setState("loading");
    try {
      const root = await native.workspaceCurrentDir();
      if (!root) {
        setWorkspace(null);
        setState("unavailable");
        return;
      }
      setWorkspace(root);
      try {
        const result = await native.readFile(projectInstructionsPath(root));
        if (result.kind === "text") {
          setContent(result.content);
          setState("ready");
        } else {
          setContent("");
          setState("error");
        }
      } catch {
        setContent("");
        setState("missing");
      }
    } catch {
      setWorkspace(null);
      setState("unavailable");
    }
  };

  useEffect(() => {
    void load();
  }, []);

  const save = async () => {
    if (!workspace || saving) return;
    setSaving(true);
    setNotice(null);
    try {
      await native.writeFile(
        projectInstructionsPath(workspace),
        content.trimEnd() + "\n",
        {
          source: "altai-project-intelligence",
        },
      );
      setState("ready");
      setNotice("Project instructions saved. New agent runs will use them.");
    } catch (cause) {
      setNotice(
        cause instanceof Error
          ? cause.message
          : "Could not save project instructions.",
      );
    } finally {
      setSaving(false);
    }
  };

  const canEdit =
    state !== "loading" && state !== "unavailable" && state !== "error";

  return (
    <section
      aria-label="Project Intelligence"
      className="shrink-0 border-b border-border/45"
    >
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-2 px-3 py-2 text-left transition-colors hover:bg-foreground/[0.03]"
      >
        <HugeiconsIcon
          icon={InformationCircleIcon}
          size={13}
          strokeWidth={1.8}
          className="shrink-0 text-muted-foreground"
        />
        <span className="flex-1 text-[9.5px] font-semibold uppercase tracking-wide text-muted-foreground/65">
          Project Intelligence
        </span>
        <HugeiconsIcon
          icon={ArrowDown01Icon}
          size={12}
          strokeWidth={2}
          className={cn(
            "shrink-0 text-muted-foreground/55 transition-transform duration-150",
            open && "rotate-180",
          )}
        />
      </button>

      {open ? (
        <div className="px-2.5 pb-3">
          <p className="mb-2 px-1 text-[10px] leading-relaxed text-muted-foreground">
            Project rules combined with the selected agent's instructions on
            every new run.
          </p>

          <div className="mb-2 flex items-center gap-1.5 px-1">
            <span className="min-w-0 flex-1 truncate text-[9.5px] text-muted-foreground/70">
              {workspace
                ? projectInstructionsPath(workspace)
                : "Open a workspace to manage its instructions."}
            </span>
            <Button
              type="button"
              size="xs"
              variant="ghost"
              className="h-6 gap-1 px-1.5 text-[9.5px]"
              onClick={() => void load()}
              disabled={state === "loading" || saving}
            >
              <HugeiconsIcon
                icon={Refresh01Icon}
                size={11}
                strokeWidth={1.75}
              />
              Refresh
            </Button>
          </div>

          {state === "loading" ? (
            <p className="px-1 py-4 text-center text-[10.5px] text-muted-foreground">
              Loading workspace context…
            </p>
          ) : null}
          {state === "unavailable" ? (
            <p className="mx-1 rounded-lg bg-muted/50 px-2.5 py-3 text-[10.5px] text-muted-foreground">
              No active workspace is available.
            </p>
          ) : null}
          {state === "error" ? (
            <p className="mx-1 rounded-lg bg-destructive/10 px-2.5 py-3 text-[10.5px] text-destructive">
              {PROJECT_INSTRUCTIONS_FILE} exists but could not be read as text.
            </p>
          ) : null}
          {state === "missing" ? (
            <div className="mx-1 rounded-lg border border-dashed border-border/70 p-3">
              <p className="text-[10px] leading-relaxed text-muted-foreground">
                No {PROJECT_INSTRUCTIONS_FILE} yet. Create a small starter here,
                or use{" "}
                <code className="rounded bg-muted px-1 py-0.5 text-[9px]">
                  /init
                </code>{" "}
                in chat to ask the agent to inspect the repository and draft it.
              </p>
              <Button
                type="button"
                size="xs"
                className="mt-2 h-6 text-[10px]"
                onClick={() => {
                  setContent(STARTER);
                  setState("ready");
                }}
              >
                Create starter
              </Button>
            </div>
          ) : null}

          {canEdit ? (
            <>
              <Textarea
                value={content}
                onChange={(event) => setContent(event.target.value)}
                className="mt-1 min-h-48 rounded-lg border-border/70 bg-background/60 font-mono text-[10.5px] leading-relaxed"
                placeholder="Project goals, commands, conventions, and constraints…"
              />
              <div className="mt-2 flex items-center gap-2 px-0.5">
                <Button
                  type="button"
                  size="xs"
                  className="h-6 gap-1.5 text-[10px]"
                  onClick={() => void save()}
                  disabled={saving || !content.trim()}
                >
                  <HugeiconsIcon
                    icon={Tick02Icon}
                    size={11}
                    strokeWidth={1.75}
                  />
                  {saving ? "Saving…" : "Save instructions"}
                </Button>
                {notice ? (
                  <span className="flex items-center gap-1 text-[9.5px] text-emerald-700 dark:text-emerald-300">
                    <HugeiconsIcon
                      icon={CheckmarkCircle02Icon}
                      size={11}
                      strokeWidth={1.75}
                    />
                    {notice}
                  </span>
                ) : null}
              </div>
            </>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
