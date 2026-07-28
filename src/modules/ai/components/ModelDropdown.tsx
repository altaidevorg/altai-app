import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { cn } from "@/lib/utils";
import { openSettingsWindow } from "@/modules/settings/openSettingsWindow";
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  AiBookIcon,
  AiBrain01Icon,
  AppleIcon,
  ArrowDown01Icon,
  ChatGptIcon,
  ClaudeIcon,
  ComputerIcon,
  CpuIcon,
  DeepseekIcon,
  FlashIcon,
  GlobeIcon,
  GoogleGeminiIcon,
  Grok02Icon,
  Hexagon01Icon,
  PlugIcon,
  Search01Icon,
  Settings01Icon,
  Tick01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  getModel,
  listAvailableModels,
  listConfiguredProviders,
  providerNeedsKey,
  type ModelId,
  type ModelInfo,
  type ProviderId,
} from "../config";
import { useChatStore } from "../store/chatStore";

const PROVIDER_ICON = {
  openai: ChatGptIcon,
  anthropic: ClaudeIcon,
  google: GoogleGeminiIcon,
  xai: Grok02Icon,
  cerebras: CpuIcon,
  groq: FlashIcon,
  deepseek: DeepseekIcon,
  mistral: Hexagon01Icon,
  zai: AiBrain01Icon,
  "zai-coding-plan": AiBrain01Icon,
  openrouter: GlobeIcon,
  "openai-compatible": PlugIcon,
  lmstudio: ComputerIcon,
  mlx: AppleIcon,
} as const satisfies Record<ProviderId, typeof ChatGptIcon>;

const MODEL_LISTBOX_ID = "model-switcher-listbox";
const modelOptionDomId = (id: string): string => `model-option-${id}`;

/** Compact gear that opens Settings → Models. Use next to any model picker. */
export function ModelSettingsButton({
  className,
  size = 14,
}: {
  className?: string;
  size?: number;
}) {
  return (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      title="Model settings"
      aria-label="Model settings"
      onClick={() => void openSettingsWindow("models")}
      className={cn(
        "size-6 shrink-0 rounded-md text-muted-foreground hover:bg-accent hover:text-foreground",
        className,
      )}
    >
      <HugeiconsIcon icon={Settings01Icon} size={size} strokeWidth={1.75} />
    </Button>
  );
}

/**
 * Chat / sidebar model picker. Only lists models whose provider has an API
 * key (or is keyless/local). Provider rail is limited to those same providers.
 * Footer always links to Settings → Models.
 */
export function ModelDropdown({
  value,
  onChange,
  className,
}: {
  /** Controlled selection — defaults to chat store `selectedModelId`. */
  value?: string;
  onChange?: (modelId: ModelId) => void;
  className?: string;
}) {
  const storeSelected = useChatStore((s) => s.selectedModelId);
  const apiKeys = useChatStore((s) => s.apiKeys);
  const setStoreSelected = useChatStore((s) => s.setSelectedModelId);
  const hiddenIds = usePreferencesStore((s) => s.hiddenModelIds);

  const selected = (value ?? storeSelected) as ModelId;
  const setSelected = (id: ModelId) => {
    if (onChange) onChange(id);
    else setStoreSelected(id);
  };

  const current = getModel(selected);
  const [search, setSearch] = useState("");
  const [activeProvider, setActiveProvider] = useState<ProviderId | null>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const [open, setOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const available = useMemo(
    () => listAvailableModels(apiKeys, hiddenIds),
    [apiKeys, hiddenIds],
  );
  const configuredProviders = useMemo(
    () => listConfiguredProviders(apiKeys, hiddenIds),
    [apiKeys, hiddenIds],
  );

  const currentUsable = available.some((m) => m.id === current.id);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    let pool = available;
    if (activeProvider !== null) {
      pool = pool.filter((m) => m.provider === activeProvider);
    }
    if (q) {
      pool = pool.filter(
        (m) =>
          m.label.toLowerCase().includes(q) ||
          m.hint.toLowerCase().includes(q) ||
          m.description.toLowerCase().includes(q) ||
          m.provider.includes(q),
      );
    }
    return pool;
  }, [activeProvider, available, search]);

  const ProviderIcon = PROVIDER_ICON[current.provider] ?? ChatGptIcon;

  useEffect(() => {
    setActiveIndex(0);
  }, [filtered]);

  useEffect(() => {
    const id = filtered[activeIndex]?.id;
    if (!id) return;
    document
      .getElementById(modelOptionDomId(id))
      ?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, filtered]);

  const pickModel = (m: ModelInfo) => {
    setSelected(m.id as ModelId);
    setOpen(false);
  };

  const handleInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setActiveIndex((i) => Math.min(filtered.length - 1, i + 1));
        break;
      case "ArrowUp":
        e.preventDefault();
        setActiveIndex((i) => Math.max(0, i - 1));
        break;
      case "Home":
        e.preventDefault();
        setActiveIndex(0);
        break;
      case "End":
        e.preventDefault();
        setActiveIndex(filtered.length - 1);
        break;
      case "Enter": {
        const m = filtered[activeIndex];
        if (m) {
          e.preventDefault();
          pickModel(m);
        }
        break;
      }
    }
  };

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (!next) {
          setSearch("");
          setActiveProvider(null);
        }
      }}
    >
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className={cn(
            "group flex h-7 min-w-0 max-w-[11rem] items-center gap-1.5 rounded-md px-2 text-[11.5px]",
            "transition-colors hover:bg-accent hover:text-foreground",
            currentUsable ? "text-foreground/80" : "text-warning",
            className,
          )}
          title={
            currentUsable
              ? `Model: ${current.label}`
              : `${current.label} — add an API key in Model settings`
          }
        >
          <HugeiconsIcon
            icon={ProviderIcon}
            size={13}
            strokeWidth={1.75}
            className="shrink-0 opacity-80"
          />
          <span className="min-w-0 truncate font-medium">{current.label}</span>
          <HugeiconsIcon
            icon={ArrowDown01Icon}
            size={11}
            strokeWidth={2}
            className="shrink-0 opacity-60 transition-opacity group-hover:opacity-90"
          />
        </Button>
      </PopoverTrigger>

      <PopoverContent
        side="top"
        align="end"
        sideOffset={6}
        collisionPadding={8}
        className="flex w-[min(20rem,calc(100vw-1rem))] flex-col gap-0 overflow-hidden rounded-xl border border-border/70 p-0 shadow-xl"
        onOpenAutoFocus={(e) => {
          e.preventDefault();
          inputRef.current?.focus();
        }}
      >
        <div className="flex items-center gap-2 border-b border-border/70 px-3 py-2">
          <HugeiconsIcon
            icon={Search01Icon}
            size={14}
            strokeWidth={1.75}
            className="shrink-0 text-muted-foreground/70"
          />
          <input
            ref={inputRef}
            role="combobox"
            aria-expanded
            aria-controls={MODEL_LISTBOX_ID}
            aria-autocomplete="list"
            aria-activedescendant={
              filtered[activeIndex]
                ? modelOptionDomId(filtered[activeIndex].id)
                : undefined
            }
            aria-label="Search models"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={handleInputKeyDown}
            placeholder="Search models…"
            className="w-full bg-transparent text-xs outline-none placeholder:text-muted-foreground/60"
          />
        </div>

        <div className="flex min-h-0">
          {configuredProviders.length > 1 ? (
            <div className="flex w-10 flex-col gap-0.5 border-r border-border/70 bg-muted/20 py-1.5">
              <ProviderPill
                icon={AiBookIcon}
                title="All providers"
                active={activeProvider === null}
                onClick={() => setActiveProvider(null)}
              />
              {configuredProviders.map((p) => (
                <ProviderPill
                  key={p.id}
                  icon={PROVIDER_ICON[p.id]}
                  title={p.label}
                  active={activeProvider === p.id}
                  onClick={() => setActiveProvider(p.id)}
                />
              ))}
            </div>
          ) : null}

          <div className="max-h-[18rem] flex-1 overflow-y-auto py-1">
            <div id={MODEL_LISTBOX_ID} role="listbox" aria-label="Models">
              {filtered.length === 0 ? (
                <div className="px-4 py-8 text-center text-xs text-muted-foreground/70">
                  {available.length === 0
                    ? "No models available — add an API key in Model settings."
                    : "No models match."}
                </div>
              ) : (
                filtered.map((m, i) => (
                  <button
                    key={m.id}
                    type="button"
                    id={modelOptionDomId(m.id)}
                    role="option"
                    aria-selected={m.id === selected}
                    data-active={i === activeIndex || undefined}
                    onClick={() => pickModel(m)}
                    className={cn(
                      "mx-1 my-0.5 flex w-[calc(100%-0.5rem)] items-center gap-2 rounded-md px-2 py-1.5 text-left",
                      m.id === selected
                        ? "bg-accent/60 text-foreground"
                        : i === activeIndex
                          ? "bg-accent/40 text-foreground"
                          : "text-foreground/85 hover:bg-accent/40 hover:text-foreground",
                    )}
                  >
                    {configuredProviders.length !== 1 ||
                    activeProvider === null ? (
                      <HugeiconsIcon
                        icon={PROVIDER_ICON[m.provider]}
                        size={13}
                        strokeWidth={1.5}
                        className="shrink-0 text-muted-foreground/70"
                      />
                    ) : null}
                    <span className="min-w-0 flex-1 truncate text-[12px] font-medium">
                      {m.label}
                    </span>
                    {m.id === selected ? (
                      <HugeiconsIcon
                        icon={Tick01Icon}
                        size={13}
                        strokeWidth={2}
                        className="shrink-0"
                      />
                    ) : null}
                  </button>
                ))
              )}
            </div>
          </div>
        </div>

        <div className="border-t border-border/70 p-1">
          <button
            type="button"
            onClick={() => {
              setOpen(false);
              void openSettingsWindow("models");
            }}
            className="flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-[12px] text-muted-foreground transition-colors hover:bg-accent/50 hover:text-foreground"
          >
            <HugeiconsIcon icon={Settings01Icon} size={12} strokeWidth={1.75} />
            Model settings…
          </button>
        </div>
      </PopoverContent>
    </Popover>
  );
}

function ProviderPill({
  icon,
  title,
  active,
  onClick,
}: {
  icon: typeof AiBookIcon;
  title: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      className={cn(
        "relative mx-auto flex size-7 items-center justify-center rounded-md transition-colors",
        active
          ? "bg-accent text-foreground after:absolute after:top-1.5 after:right-0 after:bottom-1.5 after:w-[2px] after:rounded-full after:bg-primary after:content-['']"
          : "text-muted-foreground hover:bg-accent/40 hover:text-foreground",
      )}
    >
      <HugeiconsIcon icon={icon} size={14} strokeWidth={1.5} />
    </button>
  );
}

/** @deprecated kept so callers that checked key presence still compile */
export function modelNeedsKey(provider: ProviderId): boolean {
  return providerNeedsKey(provider);
}
