import { Spinner } from "@/components/ui/spinner";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { useWorkspaceFolderStore } from "@/modules/workspace/folder";
import {
  ArrowDown01Icon,
  ArrowLeft01Icon,
  CalendarSyncIcon,
  Notebook01Icon,
  Refresh01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { type ReactNode, useEffect, useMemo, useState } from "react";
import type { AgentAutomationInfo } from "../lib/native";
import { useChatStore } from "../store/chatStore";
import { useAutomationStore } from "../store/automationStore";
import {
  AutomationCard,
  automationLastRunLabel,
  automationNextRunLabel,
  automationScheduleLabel,
  AuxiliarySurface,
  PromptTemplateGrid,
  SurfaceEmptyState,
  SurfaceFilteredEmpty,
  SurfaceIconAction,
  SurfaceSearch,
  SurfaceSectionHeader,
  SurfaceTabs,
} from "@altai/agent-ui";

type ScheduleMode = "at" | "every";
type AutomationFilter = "all" | "once" | "repeat" | "issues";

const AUTOMATION_TEMPLATES = [
  {
    label: "Code health",
    message:
      "Review the latest workspace changes for regressions, risky patterns, and missing tests. Return a concise, prioritized report with file references.",
  },
  {
    label: "Test failures",
    message:
      "Run the relevant test suite, investigate any failures, and return the root cause with the smallest safe fix or a clear recommended next step.",
  },
  {
    label: "Project brief",
    message:
      "Summarize meaningful workspace changes since the previous run. Highlight completed work, open risks, decisions, and the next three priorities.",
  },
];

function defaultAtValue(): string {
  const next = new Date(Date.now() + 5 * 60_000);
  next.setSeconds(0, 0);
  const local = new Date(next.getTime() - next.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

export function AutomationsPanel({
  onClose,
  navigation,
}: {
  onClose: () => void;
  navigation?: ReactNode;
}) {
  const workspacePath = useWorkspaceFolderStore((state) => state.folder);
  const activeChatId = useChatStore((state) => state.activeSessionId);
  const sessions = useChatStore((state) => state.sessions);
  const switchSession = useChatStore((state) => state.switchSession);
  const items = useAutomationStore((state) => state.items);
  const jobsByAutomationId = useAutomationStore((state) => state.jobsByAutomationId);
  const hydrated = useAutomationStore((state) => state.hydrated);
  const loading = useAutomationStore((state) => state.loading);
  const error = useAutomationStore((state) => state.error);
  const pendingIds = useAutomationStore((state) => state.pendingIds);
  const refresh = useAutomationStore((state) => state.refresh);
  const create = useAutomationStore((state) => state.create);
  const remove = useAutomationStore((state) => state.remove);
  const clearError = useAutomationStore((state) => state.clearError);
  const [message, setMessage] = useState("");
  const [mode, setMode] = useState<ScheduleMode>("at");
  const [atValue, setAtValue] = useState(defaultAtValue);
  const [everyMinutes, setEveryMinutes] = useState("60");
  const [ownerChatId, setOwnerChatId] = useState(activeChatId ?? "");
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<AutomationFilter>("all");
  const [removeTarget, setRemoveTarget] = useState<AgentAutomationInfo | null>(
    null,
  );
  const [viewMode, setViewMode] = useState<"list" | "create">("list");

  useEffect(() => {
    void refresh(workspacePath);
  }, [refresh, workspacePath]);

  useEffect(() => {
    if (
      activeChatId &&
      (!ownerChatId || !sessions.some((session) => session.id === ownerChatId))
    ) {
      setOwnerChatId(activeChatId);
    }
  }, [activeChatId, ownerChatId, sessions]);

  const titles = useMemo(
    () => new Map(sessions.map((session) => [session.id, session.title])),
    [sessions],
  );
  const creating = Boolean(pendingIds.create);
  const scheduledAtMs = new Date(atValue).getTime();
  const repeatMinutes = Number(everyMinutes);
  const scheduleError =
    mode === "at"
      ? !Number.isFinite(scheduledAtMs) || scheduledAtMs <= Date.now()
        ? "Choose a valid future time"
        : null
      : !Number.isFinite(repeatMinutes) || repeatMinutes < 1
        ? "Minimum interval is 1 minute"
        : null;
  const canCreate = Boolean(
    ownerChatId && message.trim() && !creating && !scheduleError,
  );
  const filterCounts = useMemo(
    () => ({
      all: items.length,
      once: items.filter((item) => item.schedule.kind === "at").length,
      repeat: items.filter((item) => item.schedule.kind !== "at").length,
      issues: items.filter((item) => jobsByAutomationId[item.id]?.lastError)
        .length,
    }),
    [items, jobsByAutomationId],
  );
  const visibleItems = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return items
      .filter((item) => {
        if (filter === "once" && item.schedule.kind !== "at") return false;
        if (filter === "repeat" && item.schedule.kind === "at") return false;
        if (filter === "issues" && !jobsByAutomationId[item.id]?.lastError) {
          return false;
        }
        if (!normalizedQuery) return true;
        return [
          item.message,
          titles.get(item.chatId) ?? "",
          automationScheduleLabel(item.schedule),
          jobsByAutomationId[item.id]?.lastError ?? "",
        ]
          .join("\n")
          .toLowerCase()
          .includes(normalizedQuery);
      })
      .sort((left, right) => {
        const leftFailed = Boolean(jobsByAutomationId[left.id]?.lastError);
        const rightFailed = Boolean(jobsByAutomationId[right.id]?.lastError);
        if (leftFailed !== rightFailed) return leftFailed ? -1 : 1;
        return nextRunAt(left) - nextRunAt(right);
      });
  }, [filter, items, jobsByAutomationId, query, titles]);

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    if (!ownerChatId || !message.trim() || scheduleError) return;
    if (mode === "at") {
      const atMs = new Date(atValue).getTime();
      if (!Number.isFinite(atMs)) return;
      void create(ownerChatId, { kind: "at", atMs }, message.trim()).then((created) => {
        if (!created) return;
        setMessage("");
        setAtValue(defaultAtValue());
        setViewMode("list");
      });
      return;
    }
    const everyMs = Number(everyMinutes) * 60_000;
    if (!Number.isFinite(everyMs)) return;
    void create(ownerChatId, { kind: "every", everyMs }, message.trim()).then((created) => {
      if (created) {
        setMessage("");
        setViewMode("list");
      }
    });
  };

  const reuseAutomation = (item: AgentAutomationInfo) => {
    setMessage(item.message);
    setOwnerChatId(item.chatId);
    if (item.schedule.kind === "at") {
      setMode("at");
      setAtValue(defaultAtValue());
    } else if (item.schedule.kind === "every") {
      setMode("every");
      setEveryMinutes(String(item.schedule.everyMs / 60_000));
    }
    setViewMode("create");
  };

  return (
    <AuxiliarySurface
      title="Work"
      eyebrow="Workspace work"
      icon={Notebook01Icon}
      subtitle={
        viewMode === "list"
          ? `${filterCounts.repeat} recurring · ${filterCounts.once} one-time`
          : "Define an instruction, owner chat, and schedule"
      }
      onClose={onClose}
      navigation={navigation}
      actions={
        <>
          {viewMode === "list" ? (
            <>
              <SurfaceIconAction
                label="Refresh automations"
                onClick={() => void refresh(workspacePath)}
                disabled={loading}
              >
                {loading ? (
                  <Spinner className="size-3.5" />
                ) : (
                  <HugeiconsIcon icon={Refresh01Icon} size={13} strokeWidth={1.75} />
                )}
              </SurfaceIconAction>
              <button
                type="button"
                onClick={() => setViewMode("create")}
                className="inline-flex h-7 items-center gap-1.5 rounded-md bg-primary px-2.5 text-[9.5px] font-semibold text-primary-foreground hover:bg-primary/85"
              >
                New schedule
              </button>
            </>
          ) : (
            <button
              type="button"
              onClick={() => setViewMode("list")}
              className="inline-flex h-7 items-center gap-1.5 rounded-md border border-border bg-muted px-2.5 text-[9.5px] font-medium text-foreground hover:bg-accent"
            >
              <HugeiconsIcon icon={ArrowLeft01Icon} size={11} strokeWidth={2} />
              Schedules
            </button>
          )}
        </>
      }
      bodyClassName="overflow-y-auto"
    >
      {viewMode === "list" ? (
        <div className="shrink-0 space-y-2 border-b border-border-subtle bg-card px-3 py-2.5">
          <SurfaceSearch
            value={query}
            onChange={setQuery}
            placeholder="Search by instruction, chat, or schedule"
            className="w-full"
          />
          <SurfaceTabs
            label="Filter automations"
            value={filter}
            onChange={(value) => setFilter(value as AutomationFilter)}
            items={[
              { id: "all", label: "All", count: filterCounts.all },
              { id: "once", label: "Once", count: filterCounts.once },
              { id: "repeat", label: "Repeat", count: filterCounts.repeat },
              { id: "issues", label: "Issues", count: filterCounts.issues },
            ]}
            className="border-0 bg-transparent p-0"
          />
        </div>
      ) : null}
      {viewMode === "create" ? (
      <form onSubmit={submit} className="min-h-0 flex-1 overflow-y-auto">
        <section className="border-b border-border-subtle px-3.5 py-3.5">
          <SurfaceSectionHeader
            title="Instruction"
            description="Keep it specific, repeatable, and easy to review."
          />
        <textarea
          value={message}
          onChange={(event) => setMessage(event.target.value)}
          maxLength={10_000}
          rows={4}
          aria-label="Automation message"
          placeholder="What should the agent do?"
          className="mt-3 w-full resize-y rounded-lg border border-border bg-muted/55 px-3 py-2.5 text-[10.5px] leading-relaxed outline-none placeholder:text-muted-foreground/70 focus:border-ring"
        />
        <PromptTemplateGrid
          templates={AUTOMATION_TEMPLATES.map((template) => ({
            label: template.label,
            value: template.message,
          }))}
          onSelect={setMessage}
          columns={3}
          density="compact"
        />
        </section>
        <section className="border-b border-border-subtle px-3.5 py-3.5">
          <SurfaceSectionHeader
            title="Schedule"
            description="Choose when this instruction should return to its chat."
          />
        <div className="mt-3 flex items-center gap-2">
          <span className="text-[10px] text-muted-foreground">Schedule</span>
          <DropdownMenu>
            <DropdownMenuTrigger className="flex items-center justify-between gap-1 rounded-md border border-border bg-card px-1.5 py-1 text-[10px] text-foreground outline-none transition-colors hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring/25">
              {mode === "at" ? "Once" : "Repeat"}
              <HugeiconsIcon icon={ArrowDown01Icon} size={10} strokeWidth={2} className="text-muted-foreground" />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              <DropdownMenuItem onClick={() => setMode("at")} className={cn(mode === "at" && "bg-foreground/[0.085]")}>Once</DropdownMenuItem>
              <DropdownMenuItem onClick={() => setMode("every")} className={cn(mode === "every" && "bg-foreground/[0.085]")}>Repeat</DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
          {mode === "at" ? (
            <input
              type="datetime-local"
              value={atValue}
              onChange={(event) => setAtValue(event.target.value)}
              aria-label="Automation run time"
              className="min-w-0 flex-1 border border-border bg-card px-1.5 py-1 text-[10px] outline-none focus:border-ring"
            />
          ) : (
            <label className="flex min-w-0 flex-1 items-center gap-1 text-[10px] text-muted-foreground">
              Every
              <input
                type="number"
                min="1"
                value={everyMinutes}
                onChange={(event) => setEveryMinutes(event.target.value)}
                aria-label="Repeat interval in minutes"
                className="w-14 border border-border bg-card px-1.5 py-1 text-[10px] outline-none focus:border-ring"
              />
              min
            </label>
          )}
        </div>
        <div className="mt-2 flex flex-wrap items-center gap-1">
          <span className="mr-1 text-[9px] font-medium uppercase tracking-wide text-muted-foreground">
            Quick set
          </span>
          <button
            type="button"
            onClick={() => {
              setMode("at");
              setAtValue(localDateTimeValue(Date.now() + 15 * 60_000));
            }}
            className="rounded-md border border-border bg-card px-2 py-0.5 text-[9.5px] text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            In 15 min
          </button>
          <button
            type="button"
            onClick={() => {
              setMode("at");
              const tomorrow = new Date();
              tomorrow.setDate(tomorrow.getDate() + 1);
              tomorrow.setHours(9, 0, 0, 0);
              setAtValue(localDateTimeValue(tomorrow.getTime()));
            }}
            className="rounded-md border border-border bg-card px-2 py-0.5 text-[9.5px] text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            Tomorrow 09:00
          </button>
          <button
            type="button"
            onClick={() => {
              setMode("every");
              setEveryMinutes("1440");
            }}
            className="rounded-md border border-border bg-card px-2 py-0.5 text-[9.5px] text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            Daily
          </button>
          <button
            type="button"
            onClick={() => {
              setMode("every");
              setEveryMinutes("10080");
            }}
            className="rounded-md border border-border bg-card px-2 py-0.5 text-[9.5px] text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            Weekly
          </button>
        </div>
        </section>
        <section className="px-3.5 py-3.5">
          <SurfaceSectionHeader
            title="Conversation"
            description="The automation continues with the context of its owning chat."
          />
        <div className="mt-3 flex items-center gap-2">
          <span className="text-[10px] text-muted-foreground">Run in</span>
          <DropdownMenu>
            <DropdownMenuTrigger className="flex min-w-0 flex-1 items-center justify-between gap-2 rounded-md border border-border bg-card px-2 py-1 text-[10px] text-foreground hover:bg-accent">
              <span className="truncate">
                {titles.get(ownerChatId) || "Select a chat"}
              </span>
              <HugeiconsIcon icon={ArrowDown01Icon} size={10} strokeWidth={2} className="shrink-0 text-muted-foreground" />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start" className="max-h-64 min-w-56 overflow-y-auto">
              {sessions.map((session) => (
                <DropdownMenuItem
                  key={session.id}
                  onClick={() => setOwnerChatId(session.id)}
                  className={cn(ownerChatId === session.id && "bg-foreground/[0.085]")}
                >
                  <span className="max-w-52 truncate">{session.title || "New chat"}</span>
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
        <div className="mt-4 flex items-center justify-between gap-2 border-t border-border-subtle pt-3">
          <span className={cn("min-w-0 truncate text-[9.5px]", scheduleError ? "text-destructive" : "text-muted-foreground")}>
            {scheduleError ?? (ownerChatId ? "Schedule is ready" : "Select a chat to create one")}
          </span>
          <button
            type="button"
            onClick={() => setViewMode("list")}
            className="ml-auto rounded-md px-2.5 py-1.5 text-[10px] font-medium text-muted-foreground hover:bg-muted hover:text-foreground"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={!canCreate}
            className="rounded-md bg-primary px-3 py-1.5 text-[10px] font-semibold text-primary-foreground disabled:opacity-45"
          >
            {creating ? "Creating…" : "Create"}
          </button>
        </div>
        </section>
      </form>
      ) : null}

      {error && viewMode === "list" ? (
        <div role="alert" className="mx-3 mt-3 border border-destructive/30 bg-destructive/[0.06] px-2 py-1.5 text-[10px] text-destructive">
          {error}
          <button type="button" onClick={clearError} aria-label="Dismiss automation error" className="ml-2 underline">
            Dismiss
          </button>
        </div>
      ) : null}

      {viewMode === "list" ? (
      <div className="min-h-0 flex-1 overflow-y-auto px-3 py-3">
        {!hydrated || loading ? (
          <div className="flex items-center gap-2 text-[10px] text-muted-foreground"><Spinner className="size-3" /> Loading automations…</div>
        ) : items.length === 0 ? (
          <SurfaceEmptyState
            icon={CalendarSyncIcon}
            title="No schedules yet"
            description="Turn a useful, repeatable instruction into background work that returns to the right chat."
            action={
              <button
                type="button"
                onClick={() => setViewMode("create")}
                className="rounded-md bg-primary px-3 py-1.5 text-[10px] font-semibold text-primary-foreground"
              >
                Create an automation
              </button>
            }
          />
        ) : visibleItems.length === 0 ? (
          <SurfaceFilteredEmpty
            message="No automations match this view."
            className="px-3 py-7 text-[10.5px]"
            onClear={() => {
              setQuery("");
              setFilter("all");
            }}
          />
        ) : (
          <section>
            <SurfaceSectionHeader
              title="Workspace schedules"
              description="Ordered by the next expected run"
              count={visibleItems.length}
              className="mb-2 px-0.5"
            />
          <ul className="overflow-hidden rounded-lg border border-border bg-card" aria-label="Workspace automations">
            {visibleItems.map((item, index) => {
              const pending = Boolean(pendingIds[`remove:${item.id}`]);
              const job = jobsByAutomationId[item.id];
              return (
                <AutomationCard
                  key={item.id}
                  className={index > 0 ? "border-t border-border-subtle" : undefined}
                  message={item.message}
                  scheduleLabel={automationScheduleLabel(item.schedule)}
                  nextRunLabel={automationNextRunLabel(item)}
                  lastRunLabel={automationLastRunLabel(item.lastRunAtMs)}
                  owningChatLabel={titles.get(item.chatId) || "Owning chat"}
                  jobState={job?.state ?? null}
                  jobError={job?.lastError ?? null}
                  pendingRemove={pending}
                  onOpenChat={() => {
                    switchSession(item.chatId);
                    onClose();
                  }}
                  onDuplicate={() => reuseAutomation(item)}
                  onRemove={() => setRemoveTarget(item)}
                />
              );
            })}
          </ul>
          </section>
        )}
      </div>
      ) : null}
      <AlertDialog
        open={removeTarget !== null}
        onOpenChange={(open) => {
          if (!open) setRemoveTarget(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Remove automation?</AlertDialogTitle>
            <AlertDialogDescription>
              Future runs will stop being scheduled. Existing chat history stays available.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="line-clamp-3 rounded-md bg-muted px-3 py-2 text-[11px] leading-relaxed text-muted-foreground">
            {removeTarget?.message}
          </div>
          <AlertDialogFooter>
            <AlertDialogCancel>Keep automation</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => {
                if (removeTarget) {
                  void remove(removeTarget.id, removeTarget.chatId);
                }
                setRemoveTarget(null);
              }}
            >
              Remove
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </AuxiliarySurface>
  );
}

function localDateTimeValue(timestamp: number): string {
  const value = new Date(timestamp);
  const local = new Date(value.getTime() - value.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

function nextRunAt(item: AgentAutomationInfo): number {
  if (item.schedule.kind === "at") return item.schedule.atMs;
  if (item.schedule.kind === "every") {
    return (item.lastRunAtMs ?? Date.now()) + item.schedule.everyMs;
  }
  return Number.MAX_SAFE_INTEGER;
}
