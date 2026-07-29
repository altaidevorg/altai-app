import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { ModelDropdown } from "@/modules/ai/components/ModelDropdown";
import { PERMISSION_MODE_LABELS } from "@/modules/settings/store";
import {
  Alert02Icon,
  CheckmarkCircle01Icon,
  Clock01Icon,
  FlashIcon,
  PlayIcon,
  Refresh01Icon,
  Robot01Icon,
  Settings01Icon,
  ShieldEnergyIcon,
  StopCircleIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useState } from "react";
import { useOrchestrationStore } from "./store";

type Permission = keyof typeof PERMISSION_MODE_LABELS;

type WorkflowDraft = {
  maxConcurrent: number;
  maxAttempts: number;
  retryBaseSeconds: number;
  retryMaxSeconds: number;
  modelId: string;
  permissionMode: Permission;
  prompt: string;
};

type Preset = {
  id: "balanced" | "fast" | "guarded" | "autonomous";
  name: string;
  description: string;
  icon: typeof Robot01Icon;
  values: Omit<WorkflowDraft, "modelId">;
};

const PRESETS: Preset[] = [
  {
    id: "balanced",
    name: "Balanced",
    description: "Two workers, measured retries, approval before risky changes.",
    icon: Robot01Icon,
    values: {
      maxConcurrent: 2,
      maxAttempts: 4,
      retryBaseSeconds: 5,
      retryMaxSeconds: 300,
      permissionMode: "ask",
      prompt:
        "Complete each assigned task end-to-end. Inspect the repository before editing, keep changes focused, run relevant verification, and report the result with remaining risks.",
    },
  },
  {
    id: "fast",
    name: "Fast lane",
    description: "More parallelism and short retry windows for routine work.",
    icon: FlashIcon,
    values: {
      maxConcurrent: 4,
      maxAttempts: 2,
      retryBaseSeconds: 3,
      retryMaxSeconds: 60,
      permissionMode: "auto-edit",
      prompt:
        "Prioritize throughput on independent tasks. Make focused changes, run the fastest relevant checks, and surface blockers immediately instead of expanding scope.",
    },
  },
  {
    id: "guarded",
    name: "Guarded",
    description: "One worker at a time with explicit approval and deeper checks.",
    icon: ShieldEnergyIcon,
    values: {
      maxConcurrent: 1,
      maxAttempts: 3,
      retryBaseSeconds: 15,
      retryMaxSeconds: 600,
      permissionMode: "ask",
      prompt:
        "Work conservatively. Inspect dependencies and existing conventions first, request approval for meaningful changes, run comprehensive verification, and document every risk.",
    },
  },
  {
    id: "autonomous",
    name: "Autonomous",
    description: "High parallelism for well-scoped queues with automatic edits.",
    icon: PlayIcon,
    values: {
      maxConcurrent: 6,
      maxAttempts: 5,
      retryBaseSeconds: 5,
      retryMaxSeconds: 180,
      permissionMode: "auto-edit",
      prompt:
        "Own each task through implementation and verification. Keep work isolated, avoid unrelated edits, retry recoverable failures, and leave every task in a reviewable state.",
    },
  },
];

const DEFAULT_DRAFT: WorkflowDraft = {
  maxConcurrent: 2,
  maxAttempts: 4,
  retryBaseSeconds: 5,
  retryMaxSeconds: 300,
  modelId: "",
  permissionMode: "ask",
  prompt: "",
};

function workflowContent(input: WorkflowDraft): string {
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

function fingerprint(input: WorkflowDraft): string {
  return JSON.stringify({
    ...input,
    modelId: input.modelId.trim(),
    prompt: input.prompt.trim(),
  });
}

function workflowError(draft: WorkflowDraft): string | null {
  if (!draft.prompt.trim()) return "Add a workflow instruction.";
  if (draft.maxConcurrent < 1 || draft.maxConcurrent > 8)
    return "Workers must be between 1 and 8.";
  if (draft.maxAttempts < 1 || draft.maxAttempts > 10)
    return "Attempts must be between 1 and 10.";
  if (draft.retryBaseSeconds < 1 || draft.retryBaseSeconds > 3600)
    return "Retry delay must be between 1 and 3600 seconds.";
  if (draft.retryMaxSeconds < draft.retryBaseSeconds)
    return "Retry cap cannot be shorter than the base delay.";
  if (draft.retryMaxSeconds > 86400)
    return "Retry cap cannot exceed 86400 seconds.";
  return null;
}

export function OrchestrationControlCenter({
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

  const [configureOpen, setConfigureOpen] = useState(false);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [draft, setDraft] = useState<WorkflowDraft>(DEFAULT_DRAFT);
  const [savedDraft, setSavedDraft] = useState<WorkflowDraft | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  useEffect(() => {
    void load(workspaceKey);
  }, [load, workspaceKey]);

  useEffect(() => {
    if (!effective) return;
    const next: WorkflowDraft = {
      maxConcurrent: effective.config.orchestration.max_concurrent,
      maxAttempts: effective.config.orchestration.max_attempts,
      retryBaseSeconds:
        effective.config.orchestration.retry_base_seconds,
      retryMaxSeconds: effective.config.orchestration.retry_max_seconds,
      modelId: effective.config.agent.model_id ?? "",
      permissionMode: effective.config.agent.permission_mode ?? "ask",
      prompt: effective.prompt,
    };
    setDraft(next);
    setSavedDraft(next);
  }, [effective, document?.modifiedAtMs]);

  const status = snapshot?.status ?? "stopped";
  const running = status === "running";
  const paused = status === "paused";
  const active =
    (snapshot?.activeCount ?? 0) + (snapshot?.claimingCount ?? 0);
  const capacity = Math.min(
    100,
    Math.round((active / Math.max(1, draft.maxConcurrent)) * 100),
  );
  const dirty = savedDraft
    ? fingerprint(savedDraft) !== fingerprint(draft)
    : false;
  const localValidationError = workflowError(draft);
  const validationError = document?.validationError ?? localValidationError;
  const activePreset = PRESETS.find(
    (preset) =>
      preset.values.maxConcurrent === draft.maxConcurrent &&
      preset.values.maxAttempts === draft.maxAttempts &&
      preset.values.retryBaseSeconds === draft.retryBaseSeconds &&
      preset.values.retryMaxSeconds === draft.retryMaxSeconds &&
      preset.values.permissionMode === draft.permissionMode &&
      preset.values.prompt === draft.prompt,
  )?.id;

  const updateDraft = <K extends keyof WorkflowDraft>(
    key: K,
    value: WorkflowDraft[K],
  ) => setDraft((current) => ({ ...current, [key]: value }));

  const applyPreset = (preset: Preset) => {
    setDraft((current) => ({
      ...current,
      ...preset.values,
    }));
    setSaveError(null);
  };

  const reset = () => {
    if (savedDraft) setDraft(savedDraft);
    setSaveError(null);
  };

  const save = async (startAfterSave = false) => {
    if (localValidationError) {
      setSaveError(localValidationError);
      return;
    }
    setSaving(true);
    setSaveError(null);
    try {
      await saveWorkflow(workspaceKey, workflowContent(draft));
      setSavedDraft(draft);
      if (startAfterSave && taskSessionId) {
        await start(workspaceKey, taskSessionId);
      }
    } catch (cause) {
      setSaveError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  };

  const startQueue = () => {
    if (!taskSessionId) return;
    void start(
      workspaceKey,
      snapshot?.taskSessionId ?? taskSessionId,
    ).catch(() => undefined);
  };

  return (
    <section className="overflow-hidden rounded-2xl border border-border/55 bg-card/55 shadow-sm">
      <div className="p-3">
        <div className="flex items-start gap-2.5">
          <span
            className={cn(
              "flex size-9 shrink-0 items-center justify-center rounded-xl ring-1 ring-inset",
              running
                ? "bg-emerald-500/12 text-emerald-500 ring-emerald-500/20"
                : paused
                  ? "bg-amber-500/12 text-amber-500 ring-amber-500/20"
                  : "bg-muted/70 text-muted-foreground ring-border/50",
            )}
          >
            <HugeiconsIcon icon={Robot01Icon} size={18} strokeWidth={1.8} />
          </span>

          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="text-[12.5px] font-semibold text-foreground">
                Orchestration
              </h2>
              <StatusBadge status={status} />
              {dirty ? (
                <span className="rounded-full bg-sky-500/10 px-1.5 py-0.5 text-[9px] font-semibold text-sky-500">
                  UNSAVED
                </span>
              ) : null}
            </div>
            <p className="mt-0.5 text-[10px] leading-relaxed text-muted-foreground/70">
              Dispatch local work with controlled parallelism, retries, and a
              workspace-specific agent contract.
            </p>
          </div>

          <Button
            size="xs"
            variant={configureOpen ? "secondary" : "outline"}
            className="h-7 gap-1.5 text-[10px]"
            onClick={() => setConfigureOpen((open) => !open)}
          >
            <HugeiconsIcon icon={Settings01Icon} size={12} strokeWidth={1.9} />
            {configureOpen ? "Close setup" : "Configure"}
          </Button>
        </div>

        <div className="mt-3 grid grid-cols-2 gap-1.5 sm:grid-cols-4">
          <Stat label="Workers" value={`${active}/${draft.maxConcurrent}`} />
          <Stat
            label="Retrying"
            value={snapshot?.retryingCount ?? 0}
            tone="warning"
          />
          <Stat
            label="Completed"
            value={snapshot?.completedCount ?? 0}
            tone="success"
          />
          <Stat
            label="Last activity"
            value={
              snapshot?.lastTickMs ? formatAge(snapshot.lastTickMs) : "—"
            }
          />
        </div>

        <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-muted/70">
          <div
            className={cn(
              "h-full rounded-full transition-all duration-300",
              running
                ? "bg-emerald-500"
                : paused
                  ? "bg-amber-500"
                  : "bg-muted-foreground/30",
            )}
            style={{ width: `${capacity}%` }}
          />
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-1.5">
          {running ? (
            <Button
              size="xs"
              variant="outline"
              className="h-7 text-[10px]"
              disabled={pending}
              onClick={() =>
                void pause(workspaceKey).catch(() => undefined)
              }
            >
              Pause dispatch
            </Button>
          ) : (
            <Button
              size="xs"
              className="h-7 gap-1.5 text-[10px]"
              disabled={
                !taskSessionId ||
                pending ||
                !effective ||
                dirty ||
                !!validationError
              }
              title={
                dirty
                  ? "Save configuration before starting"
                  : !taskSessionId
                    ? "Select a chat with a local work queue"
                    : undefined
              }
              onClick={startQueue}
            >
              {pending ? (
                <Spinner className="size-3" />
              ) : (
                <HugeiconsIcon icon={PlayIcon} size={12} strokeWidth={2} />
              )}
              {paused ? "Resume dispatch" : "Start dispatch"}
            </Button>
          )}

          {status !== "stopped" ? (
            <Button
              size="xs"
              variant="ghost"
              className="h-7 gap-1 text-[10px] text-muted-foreground"
              disabled={pending}
              onClick={() =>
                void stop(workspaceKey).catch(() => undefined)
              }
            >
              <HugeiconsIcon
                icon={StopCircleIcon}
                size={12}
                strokeWidth={1.8}
              />
              Stop & return work
            </Button>
          ) : null}

          <span className="ml-auto text-[9.5px] text-muted-foreground/60">
            {taskSessionId
              ? "Local queue connected"
              : "Select a chat to connect its work queue"}
          </span>
        </div>
      </div>

      {configureOpen ? (
        <div className="border-t border-border/45 bg-background/35 p-3">
          <div className="flex items-center justify-between gap-2">
            <div>
              <h3 className="text-[11.5px] font-semibold text-foreground">
                Workflow profile
              </h3>
              <p className="text-[9.5px] text-muted-foreground/65">
                Start from a profile, then tune every control.
              </p>
            </div>
            {dirty ? (
              <Button
                size="xs"
                variant="ghost"
                className="h-7 gap-1 text-[10px]"
                onClick={reset}
              >
                <HugeiconsIcon
                  icon={Refresh01Icon}
                  size={11}
                  strokeWidth={1.9}
                />
                Reset changes
              </Button>
            ) : null}
          </div>

          <div className="mt-2 grid gap-1.5 sm:grid-cols-2 xl:grid-cols-4">
            {PRESETS.map((preset) => (
              <button
                key={preset.id}
                type="button"
                aria-pressed={activePreset === preset.id}
                onClick={() => applyPreset(preset)}
                className={cn(
                  "flex min-w-0 items-start gap-2 rounded-xl border p-2.5 text-left transition-colors",
                  activePreset === preset.id
                    ? "border-primary/35 bg-primary/[0.07] ring-1 ring-primary/15"
                    : "border-border/50 bg-card/45 hover:border-border hover:bg-muted/35",
                )}
              >
                <span
                  className={cn(
                    "flex size-7 shrink-0 items-center justify-center rounded-lg",
                    activePreset === preset.id
                      ? "bg-accent text-foreground"
                      : "bg-muted text-muted-foreground",
                  )}
                >
                  <HugeiconsIcon
                    icon={preset.icon}
                    size={14}
                    strokeWidth={1.8}
                  />
                </span>
                <span className="min-w-0">
                  <span className="block text-[10.5px] font-semibold text-foreground">
                    {preset.name}
                  </span>
                  <span className="mt-0.5 block text-[9px] leading-relaxed text-muted-foreground/65">
                    {preset.description}
                  </span>
                </span>
              </button>
            ))}
          </div>

          <div className="mt-3 grid gap-2 lg:grid-cols-2">
            <ConfigSection
              icon={Robot01Icon}
              title="Execution"
              description="Control parallel work and the model used by workers."
            >
              <RangeField
                id="orchestration-workers"
                label="Parallel workers"
                value={draft.maxConcurrent}
                min={1}
                max={8}
                suffix="workers"
                onChange={(value) => updateDraft("maxConcurrent", value)}
              />
              <div className="block text-[10px] font-medium text-muted-foreground">
                <div className="flex items-center justify-between gap-2">
                  <span>Worker model</span>
                  {draft.modelId ? (
                    <button
                      type="button"
                      onClick={() => updateDraft("modelId", "")}
                      className="text-[9px] font-normal text-primary/80 transition-colors hover:text-primary"
                    >
                      Use chat default
                    </button>
                  ) : null}
                </div>
                <ModelDropdown
                  value={draft.modelId || undefined}
                  onChange={(modelId) => updateDraft("modelId", modelId)}
                  className="mt-1 h-8 w-full max-w-none justify-between rounded-lg border border-border/60 bg-background px-2 text-[10.5px] hover:bg-accent/40"
                />
                <span className="mt-1 block text-[8.5px] font-normal leading-relaxed text-muted-foreground/60">
                  Only models with a configured API key are shown. Add more
                  from Model settings in the dropdown.
                </span>
              </div>
              <label
                htmlFor="orchestration-permission"
                className="block text-[10px] font-medium text-muted-foreground"
              >
                Permission policy
                <select
                  id="orchestration-permission"
                  value={draft.permissionMode}
                  onChange={(event) =>
                    updateDraft(
                      "permissionMode",
                      event.target.value as Permission,
                    )
                  }
                  className="mt-1 h-8 w-full rounded-lg border border-border/60 bg-background px-2 text-[10.5px] text-foreground outline-none focus:border-ring"
                >
                  {(
                    Object.keys(PERMISSION_MODE_LABELS) as Permission[]
                  ).map((mode) => (
                    <option key={mode} value={mode}>
                      {PERMISSION_MODE_LABELS[mode]}
                    </option>
                  ))}
                </select>
              </label>
            </ConfigSection>

            <ConfigSection
              icon={Clock01Icon}
              title="Recovery"
              description="Define how failed work is retried and when it stops."
            >
              <RangeField
                id="orchestration-attempts"
                label="Maximum attempts"
                value={draft.maxAttempts}
                min={1}
                max={10}
                suffix="attempts"
                onChange={(value) => updateDraft("maxAttempts", value)}
              />
              <div className="grid grid-cols-2 gap-2">
                <NumberField
                  id="orchestration-retry-base"
                  label="Initial delay"
                  value={draft.retryBaseSeconds}
                  min={1}
                  max={3600}
                  suffix="sec"
                  onChange={(value) =>
                    updateDraft("retryBaseSeconds", value)
                  }
                />
                <NumberField
                  id="orchestration-retry-cap"
                  label="Maximum delay"
                  value={draft.retryMaxSeconds}
                  min={draft.retryBaseSeconds}
                  max={86400}
                  suffix="sec"
                  onChange={(value) =>
                    updateDraft("retryMaxSeconds", value)
                  }
                />
              </div>
              <p className="rounded-lg bg-muted/45 px-2 py-1.5 text-[9px] leading-relaxed text-muted-foreground/65">
                Retries use exponential backoff from the initial delay up to
                the maximum delay.
              </p>
            </ConfigSection>
          </div>

          <ConfigSection
            icon={CheckmarkCircle01Icon}
            title="Agent contract"
            description="Describe how every worker should inspect, implement, verify, and report."
            className="mt-2"
          >
            <label htmlFor="orchestration-prompt" className="block">
              <span className="sr-only">Workflow instruction</span>
              <Textarea
                id="orchestration-prompt"
                value={draft.prompt}
                onChange={(event) =>
                  updateDraft("prompt", event.target.value)
                }
                className="min-h-28 resize-y border-border/60 bg-background font-mono text-[10.5px] leading-relaxed"
                placeholder="Define the worker contract, verification expectations, scope boundaries, and reporting format."
              />
            </label>
            <div className="flex items-center justify-between text-[9px] text-muted-foreground/55">
              <span>Workspace-specific · stored in ALTAI local data</span>
              <span>{draft.prompt.length.toLocaleString()} characters</span>
            </div>
          </ConfigSection>

          <button
            type="button"
            onClick={() => setAdvancedOpen((open) => !open)}
            className="mt-2 flex items-center gap-1 text-[9.5px] font-medium text-muted-foreground transition-colors hover:text-foreground"
          >
            <HugeiconsIcon
              icon={Settings01Icon}
              size={11}
              strokeWidth={1.8}
            />
            {advancedOpen ? "Hide configuration preview" : "Show configuration preview"}
          </button>

          {advancedOpen ? (
            <pre className="mt-2 max-h-52 overflow-auto rounded-xl border border-border/50 bg-zinc-950 p-3 font-mono text-[9.5px] leading-relaxed text-zinc-300">
              {workflowContent(draft)}
            </pre>
          ) : null}

          {error || saveError || validationError || snapshot?.lastError ? (
            <div
              role="alert"
              className="mt-2 flex items-start gap-2 rounded-xl border border-destructive/20 bg-destructive/[0.06] px-2.5 py-2 text-[10px] text-destructive"
            >
              <HugeiconsIcon
                icon={Alert02Icon}
                size={13}
                strokeWidth={1.9}
                className="mt-px shrink-0"
              />
              <span>
                {saveError ??
                  validationError ??
                  error ??
                  snapshot?.lastError}
              </span>
            </div>
          ) : null}

          <div className="mt-3 flex flex-wrap items-center gap-1.5 border-t border-border/45 pt-3">
            <span className="text-[9.5px] text-muted-foreground/60">
              {dirty ? "Configuration has unsaved changes." : "Configuration is saved locally."}
            </span>
            <div className="ml-auto flex items-center gap-1.5">
              <Button
                size="xs"
                variant="outline"
                className="h-7 text-[10px]"
                disabled={saving || !dirty || !!localValidationError}
                onClick={() => void save(false)}
              >
                {saving ? <Spinner className="mr-1 size-3" /> : null}
                Save configuration
              </Button>
              {!running ? (
                <Button
                  size="xs"
                  className="h-7 gap-1.5 text-[10px]"
                  disabled={
                    saving ||
                    !taskSessionId ||
                    !!localValidationError
                  }
                  onClick={() => void save(true)}
                >
                  <HugeiconsIcon icon={PlayIcon} size={12} strokeWidth={2} />
                  Save & start
                </Button>
              ) : null}
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}

function StatusBadge({
  status,
}: {
  status: "running" | "paused" | "stopped";
}) {
  return (
    <span
      className={cn(
        "rounded-full px-1.5 py-0.5 text-[9px] font-semibold",
        status === "running"
          ? "bg-emerald-500/12 text-emerald-500"
          : status === "paused"
            ? "bg-amber-500/12 text-amber-500"
            : "bg-muted text-muted-foreground",
      )}
    >
      {status.toUpperCase()}
    </span>
  );
}

function Stat({
  label,
  value,
  tone = "neutral",
}: {
  label: string;
  value: string | number;
  tone?: "neutral" | "warning" | "success";
}) {
  return (
    <div className="rounded-xl border border-border/45 bg-background/45 px-2.5 py-2">
      <p className="text-[8.5px] font-medium uppercase tracking-wide text-muted-foreground/60">
        {label}
      </p>
      <p
        className={cn(
          "mt-0.5 text-[13px] font-semibold",
          tone === "warning"
            ? "text-amber-500"
            : tone === "success"
              ? "text-emerald-500"
              : "text-foreground",
        )}
      >
        {value}
      </p>
    </div>
  );
}

function ConfigSection({
  icon,
  title,
  description,
  className,
  children,
}: {
  icon: typeof Robot01Icon;
  title: string;
  description: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <section
      className={cn(
        "rounded-xl border border-border/50 bg-card/45 p-3",
        className,
      )}
    >
      <div className="flex items-start gap-2">
        <span className="flex size-7 shrink-0 items-center justify-center rounded-lg bg-muted/70 text-muted-foreground">
          <HugeiconsIcon icon={icon} size={14} strokeWidth={1.8} />
        </span>
        <div>
          <h4 className="text-[10.5px] font-semibold text-foreground">
            {title}
          </h4>
          <p className="text-[9px] leading-relaxed text-muted-foreground/60">
            {description}
          </p>
        </div>
      </div>
      <div className="mt-3 space-y-2.5">{children}</div>
    </section>
  );
}

function RangeField({
  id,
  label,
  value,
  min,
  max,
  suffix,
  onChange,
}: {
  id: string;
  label: string;
  value: number;
  min: number;
  max: number;
  suffix: string;
  onChange: (value: number) => void;
}) {
  return (
    <div className="block text-[10px] font-medium text-muted-foreground">
      <span className="flex items-center justify-between gap-2">
        <span>{label}</span>
        <span className="font-mono text-[9.5px] text-foreground">
          {value} {suffix}
        </span>
      </span>
      <input
        id={id}
        type="range"
        value={value}
        min={min}
        max={max}
        aria-label={label}
        onChange={(event) => onChange(Number(event.target.value))}
        className="mt-2 h-1.5 w-full cursor-pointer accent-primary"
      />
      <span className="mt-1 flex justify-between text-[8.5px] text-muted-foreground/45">
        <span>{min}</span>
        <span>{max}</span>
      </span>
    </div>
  );
}

function NumberField({
  id,
  label,
  value,
  min,
  max,
  suffix,
  onChange,
}: {
  id: string;
  label: string;
  value: number;
  min: number;
  max: number;
  suffix: string;
  onChange: (value: number) => void;
}) {
  return (
    <label
      htmlFor={id}
      className="text-[10px] font-medium text-muted-foreground"
    >
      {label}
      <span className="relative mt-1 block">
        <Input
          id={id}
          type="number"
          value={value}
          min={min}
          max={max}
          onChange={(event) => onChange(Number(event.target.value))}
          className="h-8 pr-9 text-[10.5px]"
        />
        <span className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-[8.5px] text-muted-foreground/55">
          {suffix}
        </span>
      </span>
    </label>
  );
}

function formatAge(timestamp: number): string {
  const seconds = Math.max(0, Math.round((Date.now() - timestamp) / 1000));
  if (seconds < 5) return "now";
  if (seconds < 60) return `${seconds}s ago`;
  return `${Math.round(seconds / 60)}m ago`;
}
