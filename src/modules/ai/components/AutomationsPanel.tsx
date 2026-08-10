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
  AutomationList,
  automationLastRunLabel,
  automationNextRunLabel,
  automationScheduleLabel,
  AutomationScheduleFields,
  AuxiliarySurface,
  ConversationOwnerSection,
  CreateFormActions,
  automationFilterCounts,
  automationMatchesFilter,
  automationMatchesQuery,
  automationNextRunAtMs,
  automationScheduleError as computeAutomationScheduleError,
  compareAutomationsForList,
  defaultAutomationAtValue,
  PromptEditorSection,
  SurfaceEmptyState,
  SurfaceFilteredEmpty,
  SurfaceFilterToolbar,
  SurfaceIconAction,
  SurfaceInlineError,
  SurfaceLoadingState,
  SurfacePrimaryAction,
  SurfaceSecondaryAction,
  sessionTitleMap,
  automationTemplatesAsMessages,
  canCreateAutomationDraft,
  nextConversationOwnerChatId,
} from "@altai/agent-ui";

type ScheduleMode = "at" | "every";
type AutomationFilter = "all" | "once" | "repeat" | "issues";

const AUTOMATION_TEMPLATES = automationTemplatesAsMessages();


export function AutomationsPanel({
  onClose,
  navigation,
  presentation = "overlay",
}: {
  onClose?: () => void;
  navigation?: ReactNode;
  presentation?: "overlay" | "embedded";
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
  const [atValue, setAtValue] = useState(defaultAutomationAtValue);
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
    const next = nextConversationOwnerChatId(
      activeChatId,
      ownerChatId,
      sessions.map((session) => session.id),
    );
    if (next) setOwnerChatId(next);
  }, [activeChatId, ownerChatId, sessions]);

  const titles = useMemo(
    () => sessionTitleMap(sessions),
    [sessions],
  );
  const creating = Boolean(pendingIds.create);
  const scheduledAtMs = new Date(atValue).getTime();
  const repeatMinutes = Number(everyMinutes);
  const scheduleError = computeAutomationScheduleError(
    mode,
    scheduledAtMs,
    repeatMinutes,
  );
  const canCreate = canCreateAutomationDraft({
    ownerChatId,
    message,
    creating,
    scheduleError,
  });
  const filterCounts = useMemo(
    () => automationFilterCounts(items, jobsByAutomationId),
    [items, jobsByAutomationId],
  );
  const visibleItems = useMemo(() => {
    return items
      .filter((item) => {
        if (!automationMatchesFilter(item, filter, jobsByAutomationId)) {
          return false;
        }
        return automationMatchesQuery(
          item,
          query,
          titles.get(item.chatId) ?? "",
          automationScheduleLabel(item.schedule),
          jobsByAutomationId[item.id]?.lastError ?? "",
        );
      })
      .sort((left, right) =>
        compareAutomationsForList(left, right, jobsByAutomationId, (l, r) =>
          automationNextRunAtMs(l) - automationNextRunAtMs(r),
        ),
      );
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
        setAtValue(defaultAutomationAtValue());
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
      setAtValue(defaultAutomationAtValue());
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
      presentation={presentation}
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
              <SurfacePrimaryAction onClick={() => setViewMode("create")}>
                New schedule
              </SurfacePrimaryAction>
            </>
          ) : (
            <SurfaceSecondaryAction onClick={() => setViewMode("list")}>
              <HugeiconsIcon icon={ArrowLeft01Icon} size={11} strokeWidth={2} />
              Schedules
            </SurfaceSecondaryAction>
          )}
        </>
      }
      bodyClassName="overflow-y-auto"
    >
      {viewMode === "list" ? (
        <SurfaceFilterToolbar
          query={query}
          onQueryChange={setQuery}
          searchPlaceholder="Search by instruction, chat, or schedule"
          tabsLabel="Filter automations"
          tabValue={filter}
          onTabChange={(value) => setFilter(value as AutomationFilter)}
          tabs={[
            { id: "all", label: "All", count: filterCounts.all },
            { id: "once", label: "Once", count: filterCounts.once },
            { id: "repeat", label: "Repeat", count: filterCounts.repeat },
            { id: "issues", label: "Issues", count: filterCounts.issues },
          ]}
        />
      ) : null}
      {viewMode === "create" ? (
      <form onSubmit={submit} className="min-h-0 flex-1 overflow-y-auto">
        <PromptEditorSection
          title="Instruction"
          description="Keep it specific, repeatable, and easy to review."
          value={message}
          onChange={setMessage}
          maxLength={10_000}
          rows={4}
          ariaLabel="Automation message"
          placeholder="What should the agent do?"
          size="automation"
          templateColumns={3}
          templateDensity="compact"
          templates={AUTOMATION_TEMPLATES.map((template) => ({
            label: template.label,
            value: template.message,
          }))}
        />
        <AutomationScheduleFields
          mode={mode}
          onModeChange={setMode}
          atValue={atValue}
          onAtValueChange={setAtValue}
          everyMinutes={everyMinutes}
          onEveryMinutesChange={setEveryMinutes}
        />
        <ConversationOwnerSection
          picker={
            <DropdownMenu>
              <DropdownMenuTrigger className="flex min-w-0 flex-1 items-center justify-between gap-2 rounded-md border border-border bg-card px-2 py-1 text-[10px] text-foreground hover:bg-accent">
                <span className="truncate">
                  {titles.get(ownerChatId) || "Select a chat"}
                </span>
                <HugeiconsIcon
                  icon={ArrowDown01Icon}
                  size={10}
                  strokeWidth={2}
                  className="shrink-0 text-muted-foreground"
                />
              </DropdownMenuTrigger>
              <DropdownMenuContent
                align="start"
                className="max-h-64 min-w-56 overflow-y-auto"
              >
                {sessions.map((session) => (
                  <DropdownMenuItem
                    key={session.id}
                    onClick={() => setOwnerChatId(session.id)}
                    className={cn(
                      ownerChatId === session.id && "bg-foreground/[0.085]",
                    )}
                  >
                    <span className="max-w-52 truncate">
                      {session.title || "New chat"}
                    </span>
                  </DropdownMenuItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          }
        >
          <CreateFormActions
            status={
              scheduleError ??
              (ownerChatId ? "Schedule is ready" : "Select a chat to create one")
            }
            statusTone={scheduleError ? "destructive" : "muted"}
            onCancel={() => setViewMode("list")}
            submitDisabled={!canCreate}
            submitLabel={creating ? "Creating…" : "Create"}
          />
        </ConversationOwnerSection>
      </form>
      ) : null}

      {error && viewMode === "list" ? (
        <SurfaceInlineError
          message={error}
          onDismiss={clearError}
          dismissAriaLabel="Dismiss automation error"
        />
      ) : null}

      {viewMode === "list" ? (
      <div className="min-h-0 flex-1 overflow-y-auto px-3 py-3">
        {!hydrated || loading ? (
          <SurfaceLoadingState density="inline">
            <Spinner className="size-3" /> Loading automations…
          </SurfaceLoadingState>
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
          <AutomationList
            items={visibleItems.map((item) => {
              const pending = Boolean(pendingIds[`remove:${item.id}`]);
              const job = jobsByAutomationId[item.id];
              return {
                id: item.id,
                message: item.message,
                scheduleLabel: automationScheduleLabel(item.schedule),
                nextRunLabel: automationNextRunLabel(item),
                lastRunLabel: automationLastRunLabel(item.lastRunAtMs),
                owningChatLabel: titles.get(item.chatId) || "Owning chat",
                jobState: job?.state ?? null,
                jobError: job?.lastError ?? null,
                pendingRemove: pending,
                onOpenChat: () => {
                  switchSession(item.chatId);
                  onClose?.();
                },
                onDuplicate: () => reuseAutomation(item),
                onRemove: () => setRemoveTarget(item),
              };
            })}
          />
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

