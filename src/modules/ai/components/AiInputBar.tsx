import { Button } from "@/components/ui/button";
import { Popover, PopoverAnchor, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Spinner } from "@/components/ui/spinner";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import {
  ArrowUpIcon,
  Attachment01Icon,
  CodeIcon,
  File01Icon,
  Mic01Icon,
  Search01Icon,
  TerminalIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useMemo, useRef, useState, type ReactElement } from "react";
import {
  AiComposer,
  ComposerAttachChips,
  ComposerFollowupBar,
  ComposerTextArea,
  ComposerToolbarIcon,
  ContextAction,
  autoresizeTextarea,
  detectAtMention,
  detectSlashOrSnippetTrigger,
  ProviderConnectBanner,
  type AtMentionRange,
  type ComposerTokenTrigger,
  filterSnippetsForPicker,
  filterWorkspacePathsForPicker,
  prependComposerInstruction,
  SEMBLE_SCOUT_SEARCH_INSTRUCTION,
} from "@altai/agent-ui";
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  ACCEPTED_FILES,
  resolveComposerEnterAction,
  useComposer,
} from "../lib/composer";
import { native } from "../lib/native";
import { useWorkspaceFiles } from "../hooks/useWorkspaceFiles";
import {
  findSlashCommands,
  refreshWorkspaceSlashCommands,
} from "../lib/slashCommands";
import { useChatStore } from "../store/chatStore";
import { useSnippetsStore } from "../store/snippetsStore";
import { AgentSwitcher } from "./AgentSwitcher";
import { FilePickerContent } from "./FilePicker";
import { ModelDropdown } from "./ModelDropdown";
import { PaperImport } from "./PaperImport";
import { PermissionModeSwitcher } from "./PermissionModeSwitcher";
import { SnippetPickerContent, type PickerItem } from "./SnippetPicker";

type SnippetTrigger = ComposerTokenTrigger;
type FileTrigger = AtMentionRange;

export function AiInputBar() {
  const c = useComposer();
  const snippets = useSnippetsStore((s) => s.snippets);
  const workspaceRoot = useChatStore((s) => s.live.getWorkspaceRoot());
  const paperImportOpen = useChatStore((s) => s.paperImportOpen);
  const agentPickerEnabled = usePreferencesStore((s) => s.agentPickerEnabled);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [commandIndexVersion, setCommandIndexVersion] = useState(0);

  const [trigger, setTrigger] = useState<SnippetTrigger | null>(null);
  const [fileTrigger, setFileTrigger] = useState<FileTrigger | null>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const [contextOpen, setContextOpen] = useState(false);
  const workspaceFiles = useWorkspaceFiles(workspaceRoot, fileTrigger !== null);

  const [fileQuery, setFileQuery] = useState("");
  useEffect(() => {
    let cancelled = false;
    void refreshWorkspaceSlashCommands(workspaceRoot).finally(() => {
      if (!cancelled) setCommandIndexVersion((version) => version + 1);
    });
    return () => {
      cancelled = true;
    };
  }, [workspaceRoot]);

  useEffect(() => {
    if (!fileTrigger) {
      setFileQuery("");
      return;
    }
    const q = fileTrigger.query;
    const t = window.setTimeout(() => setFileQuery(q), 50);
    return () => window.clearTimeout(t);
  }, [fileTrigger]);

  useEffect(() => {
    autoresizeTextarea(c.textareaRef.current);
  }, [c.value, c.textareaRef]);

  // Re-run autoresize when the textarea's container width changes (e.g. the
  // user drags the agent sidebar). Without this, wrapped lines change but the
  // forced inline `style.height` stays at the old value and the box looks
  // stuck-tall after a resize.
  useEffect(() => {
    const el = c.textareaRef.current;
    if (!el || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(() => autoresizeTextarea(el));
    ro.observe(el);
    return () => ro.disconnect();
  }, [c.textareaRef]);

  const updateTrigger = () => {
    const el = c.textareaRef.current;
    if (!el) {
      setTrigger(null);
      setFileTrigger(null);
      return;
    }
    const caret = el.selectionStart ?? 0;
    setTrigger(detectSlashOrSnippetTrigger(c.value, caret));
    setFileTrigger(detectAtMention(c.value, caret));
  };

  useEffect(updateTrigger, [c.value, c.textareaRef]);

  const filteredItems = useMemo<PickerItem[]>(() => {
    if (!trigger) return [];
    const q = trigger.query;
    const cmdItems: PickerItem[] = findSlashCommands(q).map((command) => ({
      kind: "command",
      command,
    }));
    const snipItems: PickerItem[] =
      trigger.prefix === "#"
        ? filterSnippetsForPicker(snippets, q).map((snippet) => ({
            kind: "snippet",
            snippet,
          }))
        : [];
    return [...cmdItems, ...snipItems];
  }, [commandIndexVersion, trigger, snippets]);

  const FILE_PICKER_CAP = 30;
  const filteredFiles = useMemo<string[]>(() => {
    if (!fileTrigger) return [];
    return filterWorkspacePathsForPicker(
      workspaceFiles.files,
      fileQuery,
      FILE_PICKER_CAP,
    );
  }, [fileTrigger, fileQuery, workspaceFiles.files]);

  const fileTriggerOpen = fileTrigger !== null;
  const snippetTriggerOpen = trigger !== null;
  useEffect(() => {
    setActiveIndex(0);
  }, [snippetTriggerOpen, fileTriggerOpen, fileQuery]);

  const pickerOpen = trigger !== null || fileTrigger !== null;

  const onPickItem = (item: PickerItem) => {
    if (!trigger) return;
    const before = c.value.slice(0, trigger.start);
    const afterRaw = c.value.slice(trigger.end);
    let insert = "";
    if (item.kind === "snippet") {
      const needsSpace = afterRaw.length === 0 || !/^\s/.test(afterRaw);
      insert = `#${item.snippet.handle}${needsSpace ? " " : ""}`;
      c.addSnippet(item.snippet);
    } else {
      if (trigger.prefix === "/") {
        const needsSpace = afterRaw.length === 0 || !/^\s/.test(afterRaw);
        insert = `/${item.command.name}${needsSpace ? " " : ""}`;
      } else {
        c.addCommand(item.command);
      }
    }
    const after =
      item.kind === "command" && trigger.prefix === "#"
        ? afterRaw.replace(/^\s+/, "")
        : afterRaw;
    c.setValue(`${before}${insert}${after}`);
    setTrigger(null);
    setActiveIndex(0);
    requestAnimationFrame(() => {
      const el = c.textareaRef.current;
      if (!el) return;
      const caret = before.length + insert.length;
      el.focus();
      el.setSelectionRange(caret, caret);
    });
  };

  const onPickFile = async (filePath: string) => {
    if (!fileTrigger || !workspaceRoot) return;
    const before = c.value.slice(0, fileTrigger.start);
    const after = c.value.slice(fileTrigger.end);
    c.setValue(`${before}${after}`);
    setFileTrigger(null);
    setActiveIndex(0);
    const fullPath = workspaceRoot.endsWith("/")
      ? `${workspaceRoot}${filePath}`
      : `${workspaceRoot}/${filePath}`;
    await c.attachFileByPath(fullPath);
    requestAnimationFrame(() => {
      const el = c.textareaRef.current;
      if (!el) return;
      el.focus();
      el.setSelectionRange(before.length, before.length);
    });
  };

  const pickActive = () => {
    if (fileTrigger) {
      const file = filteredFiles[activeIndex];
      if (file) void onPickFile(file);
      return;
    }
    const it = filteredItems[activeIndex];
    if (it) onPickItem(it);
  };

  const voiceLabel = c.voice.recording
    ? "Listening…"
    : c.voice.transcribing
      ? "Transcribing…"
      : null;

  const hasChips =
    c.files.length > 0 ||
    c.pickedSnippets.length > 0 ||
    c.pickedCommands.length > 0;

  const attachActiveFile = async () => {
    const path = useChatStore.getState().live.getActiveFile();
    if (!path) return;
    await c.attachFileByPath(path);
    setContextOpen(false);
  };

  const attachTerminalContext = () => {
    const output = useChatStore.getState().live.getTerminalContext();
    if (!output) return;
    c.addTextContext({ kind: "terminal", name: "Active terminal", text: output });
    setContextOpen(false);
  };

  const attachWorkingDiff = async () => {
    if (!workspaceRoot) return;
    try {
      const diff = await native.gitDiff(workspaceRoot, null, false);
      if (diff.diffText.trim()) {
        c.addTextContext({ kind: "diff", name: "Working tree diff", text: diff.diffText });
      }
    } catch (cause) {
      useChatStore.getState().addActivity({
        label: "Could not attach working-tree diff",
        detail: cause instanceof Error ? cause.message : String(cause),
        tone: "error",
      });
    } finally {
      setContextOpen(false);
    }
  };

  const attachWorkspaceMap = async () => {
    if (!workspaceRoot) return;
    await c.attachFolderByPath(workspaceRoot);
    setContextOpen(false);
  };

  const prepareSembleSearch = () => {
    c.setValue((value) =>
      prependComposerInstruction(value, SEMBLE_SCOUT_SEARCH_INSTRUCTION),
    );
    requestAnimationFrame(() => c.textareaRef.current?.focus());
  };

  return (
    <div className="altai-ai-composer-wrap w-full min-w-0 max-w-full shrink-0 bg-transparent">
      <input
        ref={fileInputRef}
        type="file"
        multiple
        accept={ACCEPTED_FILES}
        className="hidden"
        onChange={(e) => {
          void c.addFiles(e.target.files);
          e.target.value = "";
        }}
      />

      {paperImportOpen && (
        <PaperImport
          onClose={() => useChatStore.getState().setPaperImportOpen(false)}
        />
      )}

      <AiComposer
        busy={c.isBusy}
        attachments={
          hasChips ? (
            <ComposerAttachChips
              files={c.files}
              onRemoveFile={c.removeFile}
              snippets={c.pickedSnippets.map((s) => ({
                id: s.id,
                handle: s.handle,
                description: s.description,
              }))}
              onRemoveSnippet={(id) => {
                const snip = c.pickedSnippets.find((s) => s.id === id);
                c.removeSnippet(id);
                if (!snip) return;
                const re = new RegExp(`(^|\\s)#${snip.handle}\\b ?`);
                c.setValue((v) => v.replace(re, (_m, lead: string) => lead));
              }}
              commands={c.pickedCommands.map((cmd) => ({
                name: cmd.name,
                label: cmd.label,
                icon: (
                  <HugeiconsIcon
                    icon={cmd.icon}
                    size={11}
                    strokeWidth={1.75}
                    className="text-muted-foreground"
                  />
                ),
              }))}
              onRemoveCommand={(name) => c.removeCommand(name)}
              contextTokenEstimate={c.contextTokenEstimate}
            />
          ) : undefined
        }
        draft={
          <Popover open={pickerOpen}>
            <PopoverAnchor asChild>
              <div className="altai-ai-composer-input relative w-full min-w-0 px-3 pb-1 pt-2.5">
                <ComposerTextArea
                  ref={c.textareaRef}
                  value={c.value}
                  onChange={(e) => c.setValue(e.target.value)}
                  onKeyUp={updateTrigger}
                  onClick={updateTrigger}
                  onSelect={updateTrigger}
                  onKeyDown={(e) => {
                    if (pickerOpen) {
                      const items = fileTrigger ? filteredFiles : filteredItems;
                      if (e.key === "ArrowDown") {
                        e.preventDefault();
                        setActiveIndex((i) =>
                          Math.min(i + 1, Math.max(0, items.length - 1)),
                        );
                        return;
                      }
                      if (e.key === "ArrowUp") {
                        e.preventDefault();
                        setActiveIndex((i) => Math.max(0, i - 1));
                        return;
                      }
                      if (e.key === "Tab" || e.key === "Enter") {
                        if (items.length > 0) {
                          e.preventDefault();
                          pickActive();
                          return;
                        }
                      }
                      if (e.key === "Escape") {
                        e.preventDefault();
                        if (fileTrigger) {
                          const before = c.value.slice(0, fileTrigger.start);
                          const after = c.value.slice(fileTrigger.end);
                          c.setValue(`${before}${after}`);
                          setFileTrigger(null);
                        } else {
                          setTrigger(null);
                        }
                        return;
                      }
                    }
                    if (e.key === "Enter") {
                      const action = resolveComposerEnterAction({
                        availability: c.actionAvailability,
                        shiftKey: e.shiftKey,
                        modifierKey: e.metaKey || e.ctrlKey,
                      });
                      if (action) e.preventDefault();
                      if (action === "steer") c.steer();
                      else if (action === "queue") c.queueNext();
                      else if (action === "send") c.submit();
                    }
                  }}
                  placeholder={
                    c.isBusy
                      ? "Add a follow-up, steer the active run, or queue the next task…"
                      : "Describe a task or ask a follow-up…  @ files  / commands  # snippets"
                  }
                />
              </div>
            </PopoverAnchor>
            {fileTrigger ? (
              <FilePickerContent
                files={filteredFiles}
                activeIndex={activeIndex}
                indexing={workspaceFiles.indexing}
                truncated={workspaceFiles.truncated}
                hasWorkspace={workspaceRoot !== null}
                onPick={(f) => void onPickFile(f)}
                onHover={setActiveIndex}
              />
            ) : (
              <SnippetPickerContent
                items={filteredItems}
                activeIndex={activeIndex}
                onPick={onPickItem}
                onHover={setActiveIndex}
                commandPrefix={trigger?.prefix}
              />
            )}
          </Popover>
        }
        followup={
          c.canSteer || c.canQueue ? (
            <ComposerFollowupBar
              hint={
                c.isCancelling
                  ? "Cancellation requested — you can queue the next task"
                  : c.canSteer
                    ? "Enter queues next · ⌘/Ctrl+Enter steers this run"
                    : "Enter queues next · starts after the active run ends"
              }
              showSteer={c.isRunning}
              showQueue={c.isBusy}
              canSteer={c.canSteer}
              canQueue={c.canQueue}
              onSteer={c.steer}
              onQueue={c.queueNext}
              steerTitle={
                c.files.some(
                  (file) => file.kind === "image" || file.kind === "pdf",
                )
                  ? "Steering cannot include images or PDFs; use Queue next"
                  : "Apply at the active run's next safe boundary"
              }
              queueTitle="Start after the active run terminates"
            />
          ) : undefined
        }
        agentSlot={
          agentPickerEnabled ? <AgentSwitcher variant="toolbar" /> : undefined
        }
        modelSlot={<ModelDropdown allowAuto className="w-full max-w-none" />}
        tools={
          <>
            <ComposerToolbarIcon
              title="Attach file or image"
              onClick={() => fileInputRef.current?.click()}
              renderTooltip={withComposerTooltip}
            >
              <HugeiconsIcon icon={Attachment01Icon} size={14} strokeWidth={1.75} />
            </ComposerToolbarIcon>

            <Popover open={contextOpen} onOpenChange={setContextOpen}>
              <Tooltip delayDuration={350} disableHoverableContent>
                <TooltipTrigger asChild>
                  <span className="inline-flex shrink-0">
                    <PopoverTrigger asChild>
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon"
                        aria-label="Add workspace context"
                        className="size-6 shrink-0 rounded-md text-muted-foreground hover:bg-accent hover:text-foreground"
                      >
                        <HugeiconsIcon icon={CodeIcon} size={14} strokeWidth={1.75} />
                      </Button>
                    </PopoverTrigger>
                  </span>
                </TooltipTrigger>
                <TooltipContent side="top" sideOffset={6} className="text-[11px]">
                  Add workspace context
                </TooltipContent>
              </Tooltip>
              <PopoverContent side="top" align="start" sideOffset={6} className="w-56 gap-0 rounded-lg border border-border/80 bg-popover p-1.5 text-popover-foreground shadow-xl">
                <ContextAction icon={File01Icon} label="Active file" detail="Attach the file open in the editor" disabled={!workspaceRoot || !useChatStore.getState().live.getActiveFile()} onClick={() => { setContextOpen(false); void attachActiveFile(); }} />
                <ContextAction icon={Attachment01Icon} label="Workspace file map" detail="Attach a compact folder manifest" disabled={!workspaceRoot} onClick={() => { setContextOpen(false); void attachWorkspaceMap(); }} />
                <ContextAction icon={TerminalIcon} label="Active terminal" detail="Attach the latest non-private output" disabled={!useChatStore.getState().live.getTerminalContext()} onClick={() => { setContextOpen(false); attachTerminalContext(); }} />
                <ContextAction icon={CodeIcon} label="Working tree diff" detail="Attach unstaged Git changes" disabled={!workspaceRoot} onClick={() => { setContextOpen(false); void attachWorkingDiff(); }} />
              </PopoverContent>
            </Popover>

            <ComposerToolbarIcon
              title="Research with Semble Scout"
              onClick={prepareSembleSearch}
              disabled={!workspaceRoot}
              renderTooltip={withComposerTooltip}
            >
              <HugeiconsIcon icon={Search01Icon} size={14} strokeWidth={1.75} />
            </ComposerToolbarIcon>
            {c.voice.supported && (
              <ComposerToolbarIcon
                title={
                  !c.voice.hasKey
                    ? "Voice needs an OpenAI key"
                    : c.voice.recording
                      ? "Stop & transcribe"
                      : c.voice.transcribing
                        ? "Transcribing…"
                        : "Voice input"
                }
                onClick={() =>
                  c.voice.recording ? c.voice.stop() : void c.voice.start()
                }
                disabled={c.voice.transcribing || !c.voice.hasKey}
                className={cn(
                  c.voice.recording &&
                    "bg-destructive/10 text-destructive hover:bg-destructive/15 hover:text-destructive",
                )}
                renderTooltip={withComposerTooltip}
              >
                {c.voice.recording ? (
                  <span className="size-2 animate-pulse rounded-full bg-destructive" />
                ) : c.voice.transcribing ? (
                  <Spinner className="size-3" />
                ) : (
                  <HugeiconsIcon icon={Mic01Icon} size={14} strokeWidth={1.75} />
                )}
              </ComposerToolbarIcon>
            )}
          </>
        }
        permission={
          <HoverTooltip label="Permission mode">
            <PermissionModeSwitcher variant="toolbar-icon" />
          </HoverTooltip>
        }
        submit={
          c.isBusy ? (
            <Button
              type="button"
              size="sm"
              variant="secondary"
              onClick={c.stop}
              disabled={c.isCancelling}
              className="h-7 gap-1.5 rounded-md px-2.5 text-[11px]"
              aria-label={c.isCancelling ? "Cancelling" : "Stop"}
            >
              {c.isCancelling ? (
                <Spinner className="size-3" />
              ) : (
                <span className="block size-2 rounded-sm bg-foreground" />
              )}
              <span className="altai-ai-composer-submit-label">
                {c.isCancelling ? "Stopping" : "Stop"}
              </span>
            </Button>
          ) : (
            <HoverTooltip label="Send · Enter">
              <Button
                type="button"
                size="icon"
                onClick={c.submit}
                disabled={!c.canSend}
                className="size-7 rounded-md p-0 transition-all active:scale-[0.98]"
                aria-label="Send"
              >
                <HugeiconsIcon icon={ArrowUpIcon} size={13} strokeWidth={2.25} />
              </Button>
            </HoverTooltip>
          )
        }
      />

      <AnimatePresence initial={false}>
        {voiceLabel && (
          <motion.div
            key={voiceLabel}
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.12 }}
            className="mt-1 flex items-center gap-1.5 px-1.5 text-[11px] text-muted-foreground"
          >
            {c.voice.recording ? (
              <span className="size-1.5 animate-pulse rounded-full bg-destructive" />
            ) : (
              <Spinner className="size-3" />
            )}
            <span className="truncate">{voiceLabel}</span>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function withComposerTooltip(label: string, children: ReactElement) {
  return (
    <Tooltip delayDuration={350} disableHoverableContent>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side="top" sideOffset={6} className="text-[11px]">
        {label}
      </TooltipContent>
    </Tooltip>
  );
}

/** Opens only while a pointer is over the control, never on click or focus. */
function HoverTooltip({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <Tooltip delayDuration={350} disableHoverableContent>
      <TooltipTrigger asChild>
        <span className="inline-flex shrink-0">{children}</span>
      </TooltipTrigger>
      <TooltipContent side="top" sideOffset={6} className="text-[11px]">
        {label}
      </TooltipContent>
    </Tooltip>
  );
}

export type AiInputBarProps = { tabId: number };

export function AiInputBarConnect({ onAdd }: { onAdd: () => void }) {
  return <ProviderConnectBanner onAdd={onAdd} />;
}
