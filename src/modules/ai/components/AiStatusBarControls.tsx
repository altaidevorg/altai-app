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
import {
  AiOpenControl,
  CheckpointMenuPanel,
  CompactNowControl,
  IconBtn,
  aiAgentToggleTitle,
  voiceInputControlDisabled,
  voiceInputControlTitle,
  miniConversationControlTitle,
} from "@altai/agent-ui";
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
  ArchiveRestoreIcon,
  ArrowUpIcon,
  Message01Icon,
  Mic01Icon,
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
    <motion.div initial={{ y: -15 }} animate={{ y: 0 }} className="inline-flex">
      <AiOpenControl
        active={active}
        onOpen={onOpen}
        title={aiAgentToggleTitle(active, fmtShortcut(MOD_KEY, "I"))}
      />
    </motion.div>
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
          title={voiceInputControlTitle({
            hasKey: c.voice.hasKey,
            recording: c.voice.recording,
            transcribing: c.voice.transcribing,
          })}
          onClick={() =>
            c.voice.recording ? c.voice.stop() : void c.voice.start()
          }
          disabled={voiceInputControlDisabled(c.isBusy, {
            hasKey: c.voice.hasKey,
            recording: c.voice.recording,
            transcribing: c.voice.transcribing,
          })}
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
        title={miniConversationControlTitle(miniOpen)}
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
    <CompactNowControl
      onClick={() => void onClick()}
      disabled={busy || running || !active}
    />
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
        <CheckpointMenuPanel
          items={items}
          restoringId={restoring}
          onRestore={(id) => void onRestore(id)}
        />
      </PopoverContent>
    </Popover>
  );
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
