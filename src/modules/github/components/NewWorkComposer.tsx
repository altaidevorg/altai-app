import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { Textarea } from "@/components/ui/textarea";
import { useChatStore } from "@/modules/ai/store/chatStore";
import { useTodosStore } from "@/modules/ai/store/todoStore";
import type { AssignmentRunConfig } from "@/modules/github/lib/assignments";
import { useAssignmentsStore } from "@/modules/github/store/assignmentsStore";
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  CheckListIcon,
  Clock01Icon,
  PlayIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState } from "react";
import { AgentRunOptionsFields, type AgentRunOptions } from "./AgentRunOptionsFields";

type Props = {
  taskSessionId: string | null;
  onClose: () => void;
  onStarted?: (assignmentId: string) => void;
};

type Mode = "run" | "backlog" | "orchestrate";

export function NewWorkComposer({ taskSessionId, onClose, onStarted }: Props) {
  const runTask = useAssignmentsStore((state) => state.runTask);
  const addTodo = useTodosStore((state) => state.addTodo);
  const selectedModelId = useChatStore((state) => state.selectedModelId);
  const permissionMode = usePreferencesStore((state) => state.permissionMode);
  const [title, setTitle] = useState("");
  const [prompt, setPrompt] = useState("");
  const [mode, setMode] = useState<Mode>("run");
  const [options, setOptions] = useState<AgentRunOptions>({
    agentId: "",
    modelId: selectedModelId,
    permissionMode,
  });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async () => {
    const cleanPrompt = prompt.trim();
    if (!cleanPrompt) {
      setError("Describe the work before starting it.");
      return;
    }
    if (mode !== "run" && !taskSessionId) {
      setError("Select or create a chat to keep this work in the backlog.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      if (mode === "run") {
        const runConfig: AssignmentRunConfig = {
          agentId: options.agentId || undefined,
          modelId: options.modelId || undefined,
          permissionMode: options.permissionMode,
        };
        const assignmentId = await runTask({
          title,
          prompt: cleanPrompt,
          runConfig,
        });
        onStarted?.(assignmentId);
      } else {
        addTodo(taskSessionId!, {
          title: title.trim() || cleanPrompt.split("\n")[0].slice(0, 96),
          description: cleanPrompt,
        });
      }
      onClose();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="rounded-xl border border-border bg-card p-3 shadow-sm">
      <div className="flex items-start gap-2">
        <div className="flex size-7 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
          <HugeiconsIcon icon={CheckListIcon} size={15} strokeWidth={1.8} />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-2">
            <div>
              <h2 className="text-[12px] font-semibold text-foreground">New work</h2>
              <p className="text-[10px] text-muted-foreground/70">
                Start an agent now, or turn the idea into a durable queued item.
              </p>
            </div>
            <Button size="xs" variant="ghost" className="h-7 text-[10px]" onClick={onClose}>
              Close
            </Button>
          </div>

          <div className="mt-3 grid gap-2 sm:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]">
            <div className="space-y-2">
              <label htmlFor="new-work-title" className="block text-[10px] font-medium text-muted-foreground">
                Title <span className="font-normal text-muted-foreground/50">(optional)</span>
                <Input
                  id="new-work-title"
                  value={title}
                  onChange={(event) => setTitle(event.target.value)}
                  placeholder="e.g. Improve onboarding empty state"
                  className="mt-1 h-8 rounded-lg border border-border bg-muted/45 px-2.5 text-[11px] focus-visible:border-border focus-visible:ring-0"
                  disabled={busy}
                />
              </label>
              <div className="grid grid-cols-3 gap-1 rounded-lg border border-border bg-muted/25 p-1">
                {([
                  ["run", "Run now", PlayIcon],
                  ["backlog", "Backlog", CheckListIcon],
                  ["orchestrate", "Orchestrate", Clock01Icon],
                ] as const).map(([value, label, icon]) => (
                  <button
                    key={value}
                    type="button"
                    aria-pressed={mode === value}
                    onClick={() => setMode(value)}
                    disabled={busy}
                    className={`flex min-h-10 flex-col items-center justify-center gap-0.5 rounded-md px-1 text-[9.5px] transition-colors ${
                      mode === value
                        ? "bg-foreground/[0.09] text-foreground"
                        : "text-muted-foreground hover:bg-foreground/[0.055] hover:text-foreground"
                    }`}
                  >
                    <HugeiconsIcon icon={icon} size={13} strokeWidth={1.9} />
                    {label}
                  </button>
                ))}
              </div>
              <p className="text-[9.5px] leading-relaxed text-muted-foreground/65">
                {mode === "run"
                  ? "Creates a background agent session immediately."
                  : mode === "backlog"
                    ? "Saves a pending local todo for manual pickup."
                    : "Saves a pending todo for the orchestration worker."}
              </p>
            </div>

            <label htmlFor="new-work-brief" className="block text-[10px] font-medium text-muted-foreground">
              Work brief
              <Textarea
                id="new-work-brief"
                value={prompt}
                onChange={(event) => setPrompt(event.target.value)}
                placeholder="What should be changed? Include acceptance criteria, constraints, and useful context."
                className="mt-1 min-h-28 resize-y rounded-lg border border-border bg-muted/45 px-2.5 py-2 text-[11px] leading-relaxed focus-visible:border-border focus-visible:ring-0"
                disabled={busy}
                autoFocus
              />
            </label>
          </div>

          {mode === "run" ? (
            <AgentRunOptionsFields
              value={options}
              onChange={setOptions}
              disabled={busy}
              className="mt-2"
            />
          ) : null}

          {error ? (
            <p role="alert" className="mt-2 rounded-lg border border-destructive/20 bg-destructive/[0.06] px-2.5 py-1.5 text-[10px] text-destructive">
              {error}
            </p>
          ) : null}

          <div className="mt-2 flex items-center justify-end gap-1.5">
            <Button size="xs" variant="ghost" className="h-7 text-[10px]" onClick={onClose} disabled={busy}>
              Cancel
            </Button>
            <Button size="xs" className="h-7 gap-1.5 text-[10.5px]" onClick={() => void submit()} disabled={busy}>
              {busy ? <Spinner className="size-3" /> : <HugeiconsIcon icon={mode === "run" ? PlayIcon : Clock01Icon} size={12} strokeWidth={2} />}
              {mode === "run" ? "Start work" : mode === "backlog" ? "Add to backlog" : "Queue for orchestration"}
            </Button>
          </div>
        </div>
      </div>
    </section>
  );
}
