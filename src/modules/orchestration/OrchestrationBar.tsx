import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { PERMISSION_MODE_LABELS } from "@/modules/settings/store";
import { Robot01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useState } from "react";
import { useOrchestrationStore } from "./store";

type Permission = "ask" | "auto-edit" | "plan" | "bypass";

function workflowContent(input: {
  maxConcurrent: number;
  maxAttempts: number;
  retryBaseSeconds: number;
  retryMaxSeconds: number;
  modelId: string;
  permissionMode: Permission;
  prompt: string;
}): string {
  const model = input.modelId.trim();
  return [
    "---",
    "orchestration:",
    `  max_concurrent: ${input.maxConcurrent}`,
    `  max_attempts: ${input.maxAttempts}`,
    `  retry_base_seconds: ${input.retryBaseSeconds}`,
    `  retry_max_seconds: ${input.retryMaxSeconds}`,
    "agent:",
    `  model_id: ${model ? JSON.stringify(model) : "null"}`,
    `  permission_mode: ${input.permissionMode}`,
    "---",
    input.prompt.trim(),
    "",
  ].join("\n");
}

function formFingerprint(input: {
  maxConcurrent: number;
  maxAttempts: number;
  retryBaseSeconds: number;
  retryMaxSeconds: number;
  modelId: string;
  permissionMode: Permission;
  prompt: string;
}): string {
  return JSON.stringify({
    ...input,
    modelId: input.modelId.trim(),
    prompt: input.prompt.trim(),
  });
}

export function OrchestrationBar({
  workspaceKey,
  taskSessionId,
}: {
  workspaceKey: string;
  taskSessionId: string | null;
}) {
  const snapshot = useOrchestrationStore(
    (state) => state.snapshots[workspaceKey],
  );
  const document = useOrchestrationStore(
    (state) => state.workflows[workspaceKey],
  );
  const effective = useOrchestrationStore(
    (state) => state.effectiveWorkflows[workspaceKey],
  );
  const error = useOrchestrationStore(
    (state) => state.errors[workspaceKey] ?? null,
  );
  const pending = useOrchestrationStore(
    (state) => state.pending[workspaceKey] ?? false,
  );
  const load = useOrchestrationStore((state) => state.load);
  const start = useOrchestrationStore((state) => state.start);
  const pause = useOrchestrationStore((state) => state.pause);
  const stop = useOrchestrationStore((state) => state.stop);
  const saveWorkflow = useOrchestrationStore((state) => state.saveWorkflow);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [maxConcurrent, setMaxConcurrent] = useState(2);
  const [maxAttempts, setMaxAttempts] = useState(4);
  const [retryBaseSeconds, setRetryBaseSeconds] = useState(5);
  const [retryMaxSeconds, setRetryMaxSeconds] = useState(300);
  const [modelId, setModelId] = useState("");
  const [permissionMode, setPermissionMode] = useState<Permission>("ask");
  const [prompt, setPrompt] = useState("");
  const [savedFingerprint, setSavedFingerprint] = useState("");

  useEffect(() => {
    void load(workspaceKey);
  }, [load, workspaceKey]);

  useEffect(() => {
    if (!effective) return;
    setMaxConcurrent(effective.config.orchestration.max_concurrent);
    setMaxAttempts(effective.config.orchestration.max_attempts);
    setRetryBaseSeconds(effective.config.orchestration.retry_base_seconds);
    setRetryMaxSeconds(effective.config.orchestration.retry_max_seconds);
    setModelId(effective.config.agent.model_id ?? "");
    setPermissionMode(effective.config.agent.permission_mode ?? "ask");
    setPrompt(effective.prompt);
    setSavedFingerprint(
      formFingerprint({
        maxConcurrent: effective.config.orchestration.max_concurrent,
        maxAttempts: effective.config.orchestration.max_attempts,
        retryBaseSeconds:
          effective.config.orchestration.retry_base_seconds,
        retryMaxSeconds: effective.config.orchestration.retry_max_seconds,
        modelId: effective.config.agent.model_id ?? "",
        permissionMode: effective.config.agent.permission_mode ?? "ask",
        prompt: effective.prompt,
      }),
    );
  }, [effective, document?.modifiedAtMs]);

  const status = snapshot?.status ?? "stopped";
  const running = status === "running";
  const paused = status === "paused";
  const busyCount =
    (snapshot?.activeCount ?? 0) + (snapshot?.claimingCount ?? 0);
  const validationError = document?.validationError ?? null;
  const dirty =
    !!savedFingerprint &&
    savedFingerprint !==
      formFingerprint({
        maxConcurrent,
        maxAttempts,
        retryBaseSeconds,
        retryMaxSeconds,
        modelId,
        permissionMode,
        prompt,
      });

  const save = async () => {
    setSaving(true);
    setSaveError(null);
    try {
      await saveWorkflow(
        workspaceKey,
        workflowContent({
          maxConcurrent,
          maxAttempts,
          retryBaseSeconds,
          retryMaxSeconds,
          modelId,
          permissionMode,
          prompt,
        }),
      );
    } catch (cause) {
      setSaveError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="shrink-0 border-b border-border/50 bg-emerald-500/[0.035]">
      <div className="flex min-h-11 items-center gap-2 px-4 py-2">
        <span
          className={cn(
            "flex size-7 shrink-0 items-center justify-center rounded-lg",
            running
              ? "bg-emerald-500/15 text-emerald-500"
              : paused
                ? "bg-amber-500/15 text-amber-500"
                : "bg-foreground/5 text-muted-foreground",
          )}
        >
          <HugeiconsIcon icon={Robot01Icon} size={15} strokeWidth={1.8} />
        </span>
        <div className="min-w-0">
          <p className="text-[11.5px] font-semibold text-foreground">
            Orchestration
            <span
              className={cn(
                "ml-2 text-[10px] font-medium",
                running
                  ? "text-emerald-500"
                  : paused
                    ? "text-amber-500"
                    : "text-muted-foreground/60",
              )}
            >
              {running ? "Running" : paused ? "Paused" : "Stopped"}
            </span>
          </p>
          <p className="truncate text-[10px] text-muted-foreground/60">
            Local todos · {busyCount} active
            {(snapshot?.retryingCount ?? 0) > 0
              ? ` · ${snapshot?.retryingCount} retrying`
              : ""}
            {` · ${maxConcurrent} agents · ${maxAttempts} attempts`}
          </p>
        </div>

        <div className="ml-auto flex items-center gap-1.5">
          <Button
            size="xs"
            variant="ghost"
            className="h-7 text-[10.5px]"
            onClick={() => setSettingsOpen((open) => !open)}
          >
            {settingsOpen ? "Hide settings" : "Configure"}
          </Button>
          {running ? (
            <Button
              size="xs"
              variant="outline"
              className="h-7 text-[10.5px]"
              disabled={pending}
              onClick={() => void pause(workspaceKey).catch(() => undefined)}
            >
              Pause
            </Button>
          ) : (
            <Button
              size="xs"
              className="h-7 text-[10.5px]"
              disabled={
                !taskSessionId ||
                pending ||
                !effective ||
                dirty ||
                !!validationError
              }
              title={
                dirty
                  ? "Save WORKFLOW.md before starting orchestration"
                  : taskSessionId
                  ? "Assign pending local todos automatically"
                  : "Create or select a chat first"
              }
              onClick={() => {
                if (!taskSessionId) return;
                void start(
                  workspaceKey,
                  snapshot?.taskSessionId ?? taskSessionId,
                ).catch(() => undefined);
              }}
            >
              {pending ? <Spinner className="mr-1 size-3" /> : null}
              {paused ? "Resume" : "Start orchestration"}
            </Button>
          )}
          {status !== "stopped" ? (
            <Button
              size="xs"
              variant="ghost"
              className="h-7 text-[10.5px] text-muted-foreground"
              disabled={pending}
              onClick={() => void stop(workspaceKey).catch(() => undefined)}
            >
              Stop all
            </Button>
          ) : null}
        </div>
      </div>

      {settingsOpen ? (
        <div className="border-t border-border/40 bg-background/55 px-4 py-3">
          <div className="grid grid-cols-2 gap-2 lg:grid-cols-6">
            <NumberField
              label="Max agents"
              value={maxConcurrent}
              min={1}
              max={8}
              onChange={setMaxConcurrent}
            />
            <NumberField
              label="Max attempts"
              value={maxAttempts}
              min={1}
              max={10}
              onChange={setMaxAttempts}
            />
            <NumberField
              label="Retry base (sec)"
              value={retryBaseSeconds}
              min={1}
              max={3600}
              onChange={setRetryBaseSeconds}
            />
            <NumberField
              label="Retry cap (sec)"
              value={retryMaxSeconds}
              min={retryBaseSeconds}
              max={86400}
              onChange={setRetryMaxSeconds}
            />
            <label className="text-[10px] font-medium text-muted-foreground">
              Permission
              <select
                value={permissionMode}
                onChange={(event) =>
                  setPermissionMode(event.target.value as Permission)
                }
                className="mt-1 h-8 w-full rounded-md border border-border/60 bg-background px-2 text-[10.5px] text-foreground"
              >
                {(Object.keys(PERMISSION_MODE_LABELS) as Permission[]).map(
                  (mode) => (
                    <option key={mode} value={mode}>
                      {PERMISSION_MODE_LABELS[mode]}
                    </option>
                  ),
                )}
              </select>
            </label>
            <label
              htmlFor="orchestration-model-override"
              className="text-[10px] font-medium text-muted-foreground"
            >
              Model override
              <Input
                id="orchestration-model-override"
                value={modelId}
                onChange={(event) => setModelId(event.target.value)}
                placeholder="Use current model"
                className="mt-1 h-8 text-[10.5px]"
              />
            </label>
          </div>
          <label
            htmlFor="orchestration-workflow-prompt"
            className="mt-2 block text-[10px] font-medium text-muted-foreground"
          >
            Agent workflow prompt
            <Textarea
              id="orchestration-workflow-prompt"
              value={prompt}
              onChange={(event) => setPrompt(event.target.value)}
              className="mt-1 min-h-24 resize-y rounded-lg border-border/60 bg-background font-mono text-[10.5px]"
            />
          </label>
          <div className="mt-2 flex items-center gap-2">
            <Button
              size="xs"
              className="h-7 text-[10.5px]"
              disabled={saving || !prompt.trim()}
              onClick={() => void save()}
            >
              {saving ? <Spinner className="mr-1 size-3" /> : null}
              {document?.exists ? "Save WORKFLOW.md" : "Create WORKFLOW.md"}
            </Button>
            <span className="truncate text-[10px] text-muted-foreground/55">
              {document?.path ?? "Loading workflow…"} · saved changes apply live
              {dirty ? " · unsaved changes" : ""}
            </span>
          </div>
        </div>
      ) : null}

      {error || saveError || validationError || snapshot?.lastError ? (
        <div
          role="alert"
          className="border-t border-destructive/15 bg-destructive/[0.06] px-4 py-1.5 text-[10.5px] text-destructive"
        >
          {saveError ?? validationError ?? error ?? snapshot?.lastError}
        </div>
      ) : null}
    </div>
  );
}

function NumberField({
  label,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="text-[10px] font-medium text-muted-foreground">
      {label}
      <Input
        type="number"
        value={value}
        min={min}
        max={max}
        onChange={(event) => onChange(Number(event.target.value))}
        className="mt-1 h-8 text-[10.5px]"
      />
    </label>
  );
}
