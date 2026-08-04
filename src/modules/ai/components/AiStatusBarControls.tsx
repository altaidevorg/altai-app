import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Kbd } from "@/components/ui/kbd";
import { Spinner } from "@/components/ui/spinner";
import { fmtShortcut, MOD_KEY } from "@/lib/platform";
import { cn } from "@/lib/utils";
import { IconBtn } from "@altai/agent-ui";
import {
  Context,
  ContextContent,
  ContextContentBody,
  ContextContentFooter,
  ContextContentHeader,
  ContextCacheUsage,
  ContextInputUsage,
  ContextOutputUsage,
  ContextTrigger,
} from "@/components/ai-elements/context";
import {
  Add01Icon,
  Archive02Icon,
  ArchiveRestoreIcon,
  ArrowUpIcon,
  Message01Icon,
  Mic01Icon,
  SidebarRightIcon,
  StopCircleIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { motion } from "motion/react";
import { useEffect, useRef, useState } from "react";
import type { LanguageModelUsage } from "ai";
import { getModelContextLimit } from "../config";
import { ACCEPTED_FILES, useComposer } from "../lib/composer";
import { native, type CheckpointInfo } from "../lib/native";
import { runCompactNow } from "../lib/slashCommands";
import { useChatStore } from "../store/chatStore";
import { ModelDropdown } from "./ModelDropdown";

export { ModelDropdown, ModelSettingsButton } from "./ModelDropdown";

export function AiOpenButton({
  onOpen,
  active = false,
}: {
  onOpen: () => void;
  active?: boolean;
}) {
  return (
    <motion.button
      initial={{ y: -15 }}
      animate={{ y: 0 }}
      type="button"
      onClick={onOpen}
      className={cn(
        "inline-flex size-6 items-center justify-center rounded-md transition-colors",
        active
          ? "bg-accent text-foreground"
          : "text-muted-foreground hover:bg-accent hover:text-foreground",
      )}
      aria-label={active ? "Hide AI agent" : "Show AI agent"}
      aria-pressed={active}
      title={`${active ? "Hide" : "Show"} AI agent  ${fmtShortcut(MOD_KEY, "I")}`}
    >
      <HugeiconsIcon icon={SidebarRightIcon} size={14} strokeWidth={1.75} />
    </motion.button>
  );
}

export function AiStatusBarControls() {
  const c = useComposer();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const openMini = useChatStore((s) => s.openMini);
  const miniOpen = useChatStore((s) => s.mini.open);
  const closePanel = useChatStore((s) => s.closePanel);

  return (
    <div className="flex items-center gap-0.5">
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

      <IconBtn
        title="Attach file or image"
        onClick={() => fileInputRef.current?.click()}
        disabled={c.isBusy}
      >
        <HugeiconsIcon icon={Add01Icon} size={13} strokeWidth={2} />
      </IconBtn>

      {c.voice.supported && (
        <IconBtn
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
          disabled={c.isBusy || c.voice.transcribing || !c.voice.hasKey}
          className={cn(
            c.voice.recording &&
              "bg-destructive/10 text-destructive hover:bg-destructive/15",
          )}
        >
          {c.voice.recording ? (
            <span className="size-2 animate-pulse rounded-full bg-destructive" />
          ) : c.voice.transcribing ? (
            <Spinner className="size-3" />
          ) : (
            <HugeiconsIcon icon={Mic01Icon} size={13} strokeWidth={1.75} />
          )}
        </IconBtn>
      )}

      <ModelDropdown />

      <ContextMeter />

      <CompactNowButton />

      <CheckpointButton />

      <span className="mx-1 h-8 w-px bg-border" aria-hidden />
      <Button
        onClick={closePanel}
        title="Close AI panel"
        size="xs"
        variant="ghost"
        aria-label="Close AI panel"
        className="px-1 text-[11px] text-foreground/85"
      >
        <Kbd className="h-4 gap-px px-2 font-mono text-[11px]">
          {fmtShortcut(MOD_KEY, "I")}
        </Kbd>
      </Button>
      <IconBtn
        title={miniOpen ? "Mini-window open" : "Open conversation"}
        onClick={openMini}
        disabled={miniOpen}
      >
        <HugeiconsIcon icon={Message01Icon} size={13} strokeWidth={1.75} />
      </IconBtn>

      {c.isBusy ? (
        <Button
          type="button"
          size="icon"
          variant="ghost"
          onClick={c.stop}
          className="size-6"
          aria-label="Stop"
          title="Stop"
        >
          <HugeiconsIcon icon={StopCircleIcon} size={13} strokeWidth={1.75} />
        </Button>
      ) : (
        <Button
          type="button"
          size="icon"
          onClick={c.submit}
          disabled={!c.canSend}
          className="ml-1 h-5.5 w-7.5"
          aria-label="Send"
          title="Send (Enter)"
        >
          <HugeiconsIcon icon={ArrowUpIcon} size={13} strokeWidth={1.75} />
        </Button>
      )}
    </div>
  );
}

/**
 * Manual "Compact now" affordance. Fires `/compact` directly (no input
 * prefill, no Enter required) — surfaces the between-turns context-condense
 * tool from a single click. Kept compact so it fits the status-bar row
 * alongside the model dropdown and the context meter.
 */
function CompactNowButton() {
  const busy = useChatStore((s) => s.agentMeta.status) !== "idle";
  const [running, setRunning] = useState(false);
  const active = useChatStore((s) => s.activeSessionId);

  const onClick = async () => {
    if (busy || running || !active) return;
    setRunning(true);
    try {
      await runCompactNow();
    } finally {
      setRunning(false);
    }
  };

  return (
    <IconBtn
      title="Compact context (run /compact now)"
      onClick={() => void onClick()}
      disabled={busy || running || !active}
    >
      <HugeiconsIcon icon={Archive02Icon} size={13} strokeWidth={1.75} />
    </IconBtn>
  );
}

/**
 * One-step undo for agent edits. Lists pre-edit checkpoints (newest first) and
 * restores the selected file to its pre-edit state. Empty when no edits have
 * been made yet or checkpointing is disabled in the workspace config.
 */
function CheckpointButton() {
  const [open, setOpen] = useState(false);
  const [items, setItems] = useState<CheckpointInfo[]>([]);
  const [restoring, setRestoring] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    let mounted = true;
    void native.checkpointList().then((list) => {
      if (mounted) setItems(list);
    });
    return () => {
      mounted = false;
    };
  }, [open]);

  const onRestore = async (id: string) => {
    if (restoring) return;
    setRestoring(id);
    try {
      await native.checkpointRestore(id);
      const list = await native.checkpointList();
      setItems(list);
    } finally {
      setRestoring(null);
    }
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <IconBtn
          title="Edit checkpoints (undo agent edits)"
          onClick={() => setOpen(true)}
        >
          <HugeiconsIcon
            icon={ArchiveRestoreIcon}
            size={13}
            strokeWidth={1.75}
          />
        </IconBtn>
      </PopoverTrigger>
      <PopoverContent
        side="top"
        align="end"
        sideOffset={6}
        className="w-[min(20rem,calc(100vw-1rem))] overflow-hidden rounded-lg border border-border/70 p-0 shadow-xl"
      >
        <div className="border-b border-border/70 px-3 py-2.5">
          <div className="text-[12px] font-medium">Edit checkpoints</div>
          <div className="text-[11px] text-muted-foreground">
            Restore files to their state before the agent edited them.
          </div>
        </div>
        <div className="max-h-[16rem] overflow-y-auto">
          {items.length === 0 ? (
            <div className="px-3 py-6 text-center text-[11px] text-muted-foreground">
              No checkpoints yet. The runtime saves one before each edit.
            </div>
          ) : (
            <ul className="divide-y divide-border/40">
              {items.map((c) => (
                <li
                  key={c.id}
                  className="flex items-center gap-2 px-3 py-2 hover:bg-muted/50"
                >
                  <div className="min-w-0 flex-1">
                    <div
                      className="truncate text-[11px] font-medium"
                      title={c.path}
                    >
                      {basename(c.path)}
                    </div>
                    <div className="flex items-center gap-1.5 text-[10px] text-muted-foreground">
                      <span>{c.label}</span>
                      <span>·</span>
                      <span>{fmtTimeAgo(c.createdMs)}</span>
                    </div>
                  </div>
                  <Button
                    type="button"
                    size="xs"
                    variant="secondary"
                    disabled={restoring === c.id}
                    onClick={() => void onRestore(c.id)}
                    className="h-6 text-[10.5px]"
                  >
                    {restoring === c.id ? "Restoring…" : "Restore"}
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}

function basename(p: string): string {
  const i = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  return i >= 0 ? p.slice(i + 1) : p;
}

function fmtTimeAgo(ms: number): string {
  const secs = Math.floor((Date.now() - ms) / 1000);
  if (secs < 60) return "just now";
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

/**
 * Context-usage meter + token/cost breakdown using the shared ai-elements
 * Context card. Shows a compact percentage trigger; hovering reveals input /
 * output / cache token counts with per-category cost estimates when tokenlens
 * has pricing for the active model.
 */
function ContextMeter() {
  const selected = useChatStore((s) => s.selectedModelId);
  const tokens = useChatStore((s) => s.agentMeta.tokens);
  const usedTokens = tokens.inputTokens;
  const maxTokens = getModelContextLimit(selected);

  if (usedTokens <= 0) return null;

  const cached = tokens.cachedInputTokens;
  const noCache = Math.max(0, usedTokens - cached);
  const usage: LanguageModelUsage = {
    inputTokens: usedTokens,
    outputTokens: tokens.outputTokens,
    totalTokens: usedTokens + tokens.outputTokens,
    cachedInputTokens: cached,
    reasoningTokens: undefined,
    inputTokenDetails: {
      noCacheTokens: noCache,
      cacheReadTokens: cached,
      cacheWriteTokens: undefined,
    },
    outputTokenDetails: {
      textTokens: tokens.outputTokens,
      reasoningTokens: undefined,
    },
  };

  return (
    <Context
      usedTokens={usedTokens}
      maxTokens={maxTokens}
      usage={usage}
      modelId={selected}
    >
      <ContextTrigger className="h-7 px-1.5 text-[11px]" />
      <ContextContent>
        <ContextContentHeader />
        <ContextContentBody>
          <ContextInputUsage />
          <ContextOutputUsage />
          <ContextCacheUsage />
        </ContextContentBody>
        <ContextContentFooter />
      </ContextContent>
    </Context>
  );
}
