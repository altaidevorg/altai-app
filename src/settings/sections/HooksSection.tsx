import { Button } from "@/components/ui/button";
import { currentWorkspaceEnv } from "@/modules/workspace";
import { currentWorkspaceFolder } from "@/modules/workspace/folder";
import {
  Alert02Icon,
  LockIcon,
  Refresh01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { SectionHeader } from "../components/SectionHeader";

type HookEvent =
  | "session_start"
  | "before_tool"
  | "after_tool"
  | "before_edit"
  | "after_edit"
  | "before_apply"
  | "after_run"
  | "on_error"
  | "before_cleanup";

type HookInspectionEntry = {
  source: "managed" | "project";
  event: HookEvent;
  command: string;
  timeoutSeconds: number;
  blocking: boolean;
  locked: boolean;
};

type HookInspection = {
  workspacePath: string;
  workflowPath: string;
  validationError: string | null;
  hooks: HookInspectionEntry[];
};

const EVENT_LABELS: Record<HookEvent, string> = {
  session_start: "Session start",
  before_tool: "Before tool",
  after_tool: "After tool",
  before_edit: "Before edit",
  after_edit: "After edit",
  before_apply: "Before apply",
  after_run: "After run",
  on_error: "On error",
  before_cleanup: "Before cleanup",
};

export function HooksSection() {
  const [inspection, setInspection] = useState<HookInspection | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    const workspaceKey = currentWorkspaceFolder();
    if (!workspaceKey) {
      setInspection(null);
      setError("Open a workspace to inspect its lifecycle hooks.");
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<HookInspection>("orchestration_hooks_inspect", {
        workspaceKey,
        workspace: currentWorkspaceEnv(),
      });
      setInspection(result);
    } catch (cause) {
      setInspection(null);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="flex flex-col gap-6">
      <SectionHeader
        title="Lifecycle Hooks"
        description="Inspect the effective project and managed commands that run around agent lifecycle events. This surface is read-only; project hooks are edited in WORKFLOW.md."
      />

      <section className="rounded-xl border border-amber-500/30 bg-amber-500/5 p-4">
        <div className="flex items-start gap-2.5">
          <HugeiconsIcon
            icon={Alert02Icon}
            size={15}
            strokeWidth={1.75}
            className="mt-0.5 shrink-0 text-amber-600 dark:text-amber-400"
          />
          <div>
            <h3 className="text-[12px] font-medium">Hooks are trusted local commands</h3>
            <p className="mt-1 text-[11px] leading-relaxed text-muted-foreground">
              ALTAI validates the assigned workspace and starts hooks there with a minimal
              environment. Commands still run with your operating-system account permissions;
              review repository hook configuration before enabling automation.
            </p>
          </div>
        </div>
      </section>

      <section className="rounded-xl border border-border/60 bg-card/60 p-5">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <h3 className="text-[12.5px] font-medium">Effective hooks</h3>
            <p className="mt-1 truncate text-[10.5px] text-muted-foreground">
              {inspection?.workflowPath ?? "WORKFLOW.md"}
            </p>
          </div>
          <Button
            type="button"
            size="sm"
            variant="ghost"
            className="h-7 gap-1.5 text-[10.5px]"
            onClick={() => void load()}
            disabled={loading}
          >
            <HugeiconsIcon icon={Refresh01Icon} size={12} strokeWidth={1.75} />
            Refresh
          </Button>
        </div>

        {loading ? (
          <p className="py-10 text-center text-[11px] text-muted-foreground">
            Inspecting lifecycle hooks…
          </p>
        ) : null}

        {!loading && error ? (
          <p className="mt-4 rounded-lg bg-destructive/10 px-3 py-4 text-[11px] text-destructive">
            {error}
          </p>
        ) : null}

        {!loading && inspection?.validationError ? (
          <p className="mt-4 rounded-lg bg-destructive/10 px-3 py-4 text-[11px] text-destructive">
            WORKFLOW.md is invalid: {inspection.validationError}
          </p>
        ) : null}

        {!loading &&
        inspection &&
        !inspection.validationError &&
        inspection.hooks.length === 0 ? (
          <p className="mt-4 rounded-lg border border-dashed border-border/70 px-3 py-8 text-center text-[11px] text-muted-foreground">
            No lifecycle hooks are configured for this workspace.
          </p>
        ) : null}

        {!loading && inspection && inspection.hooks.length > 0 ? (
          <div className="mt-4 flex flex-col gap-2">
            {inspection.hooks.map((hook, index) => (
              <article
                key={`${hook.source}:${hook.event}:${hook.command}:${index}`}
                className="rounded-lg border border-border/60 bg-background/50 p-3"
              >
                <div className="flex flex-wrap items-center gap-2 text-[10px]">
                  <span className="rounded bg-muted px-1.5 py-0.5 font-medium text-foreground">
                    {EVENT_LABELS[hook.event]}
                  </span>
                  <span className="text-muted-foreground">
                    {hook.blocking ? "Blocking" : "Observability"}
                  </span>
                  <span className="text-muted-foreground">{hook.timeoutSeconds}s timeout</span>
                  {hook.locked ? (
                    <span className="ml-auto flex items-center gap-1 text-amber-700 dark:text-amber-300">
                      <HugeiconsIcon icon={LockIcon} size={10} strokeWidth={1.75} />
                      Managed
                    </span>
                  ) : (
                    <span className="ml-auto text-muted-foreground">Project</span>
                  )}
                </div>
                <code className="mt-2 block overflow-x-auto rounded bg-muted/60 px-2.5 py-2 text-[10.5px] leading-relaxed text-foreground">
                  {hook.command}
                </code>
              </article>
            ))}
          </div>
        ) : null}
      </section>
    </div>
  );
}
