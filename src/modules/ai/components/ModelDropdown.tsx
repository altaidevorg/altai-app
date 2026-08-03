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
import { pushRecentModel, toggleFavoriteModel } from "../lib/modelPrefs";
import {
  describeModelConstraint,
  pickAutoModel,
  supportsAgentModel,
} from "../lib/modelRouting";
import { useAgentsStore } from "../store/agentsStore";
import { useChatStore } from "../store/chatStore";
import { ComposerConfigTrigger } from "@altai/agent-ui";

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
  allowAuto = false,
}: {
  /** Controlled selection — defaults to chat store `selectedModelId`. */
  value?: string;
  onChange?: (modelId: ModelId) => void;
  className?: string;
  /** Enable the task-aware Auto option in the main chat composer. */
  allowAuto?: boolean;
}) {
  const storeSelected = useChatStore((s) => s.selectedModelId);
  const apiKeys = useChatStore((s) => s.apiKeys);
  const setStoreSelected = useChatStore((s) => s.setSelectedModelId);
  const autoModelEnabled = useChatStore((s) => s.autoModelEnabled);
  const setAutoModelEnabled = useChatStore((s) => s.setAutoModelEnabled);
  const hiddenIds = usePreferencesStore((s) => s.hiddenModelIds);
  const favoriteModelIds = usePreferencesStore((s) => s.favoriteModelIds);
  const recentModelIds = usePreferencesStore((s) => s.recentModelIds);
  const activeAgentId = useAgentsStore((s) => s.activeId);
  const activeAgent = useAgentsStore((s) =>
    s.all().find((agent) => agent.id === activeAgentId),
  );

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
    return pool.filter((model) => supportsAgentModel(model, activeAgent));
  }, [activeAgent, activeProvider, available, search]);

  const autoModel = useMemo(
    () => pickAutoModel({ models: available, agent: activeAgent }),
    [activeAgent, available],
  );
  const showAuto = allowAuto && !onChange;
  const autoSelected = showAuto && autoModelEnabled;
  const triggerUsable = autoSelected ? Boolean(autoModel) : currentUsable;
  const favoriteSet = useMemo(() => new Set(favoriteModelIds), [favoriteModelIds]);
  const recentSet = useMemo(() => new Set(recentModelIds), [recentModelIds]);
  const showSections = !search.trim() && activeProvider === null;
  const autoOptionVisible = showAuto && showSections;
  const pinned = showSections
    ? favoriteModelIds
        .map((id) => filtered.find((model) => model.id === id))
        .filter((model): model is ModelInfo => Boolean(model))
    : [];
  const recent = showSections
    ? recentModelIds
        .map((id) => filtered.find((model) => model.id === id))
        .filter((model): model is ModelInfo => Boolean(model))
        .filter((model) => !favoriteSet.has(model.id))
    : [];
  const remaining = showSections
    ? filtered.filter((model) => !favoriteSet.has(model.id) && !recentSet.has(model.id))
    : filtered;

  const ProviderIcon = PROVIDER_ICON[current.provider] ?? ChatGptIcon;

  useEffect(() => {
    setActiveIndex(0);
  }, [filtered, autoOptionVisible]);

  useEffect(() => {
    const id = filtered[activeIndex - (autoOptionVisible ? 1 : 0)]?.id;
    if (!id) return;
    document
      .getElementById(modelOptionDomId(id))
      ?.scrollIntoView({ block: "nearest" });
  }, [activeIndex, filtered, autoOptionVisible]);

  const pickModel = (m: ModelInfo) => {
    if (showAuto) setAutoModelEnabled(false);
    setSelected(m.id as ModelId);
    void pushRecentModel(m.id);
    setOpen(false);
  };

  const pickAuto = () => {
    if (!autoModel) return;
    setAutoModelEnabled(true);
    setOpen(false);
  };

  const handleInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setActiveIndex((i) =>
          Math.min(filtered.length - 1 + (autoOptionVisible ? 1 : 0), i + 1),
        );
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
        setActiveIndex(filtered.length - 1 + (autoOptionVisible ? 1 : 0));
        break;
      case "Enter": {
        if (autoOptionVisible && activeIndex === 0) {
          e.preventDefault();
          pickAuto();
          break;
        }
        const m = filtered[activeIndex - (autoOptionVisible ? 1 : 0)];
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
        <ComposerConfigTrigger
          icon={
            <HugeiconsIcon
              icon={ProviderIcon}
              size={13}
              strokeWidth={1.75}
              className="shrink-0 opacity-80"
            />
          }
          label={autoSelected ? `Auto · ${autoModel?.label ?? current.label}` : current.label}
          className={cn(
            triggerUsable ? "text-foreground/85" : "text-warning",
            className,
          )}
          title={
            autoSelected
              ? `Auto selects a compatible model for each task. Current recommendation: ${autoModel?.label ?? current.label}`
              : triggerUsable
              ? `Model: ${current.label}`
              : `${current.label} — add an API key in Model settings`
          }
        />
      </PopoverTrigger>

      <PopoverContent
        side="top"
        align="end"
        sideOffset={6}
        collisionPadding={8}
        className="flex w-[min(20rem,calc(100vw-1rem))] flex-col gap-0 overflow-hidden rounded-lg border border-border/80 bg-popover p-0 text-popover-foreground shadow-xl"
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
              filtered[activeIndex - (autoOptionVisible ? 1 : 0)]
                ? modelOptionDomId(filtered[activeIndex - (autoOptionVisible ? 1 : 0)].id)
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
                    : describeModelConstraint(activeAgent) ?? "No models match."}
                </div>
              ) : (
                <>
                  {autoOptionVisible ? (
                    <ModelOption
                      model={autoModel ?? current}
                      label="Auto"
                      detail={autoModel ? `Recommended now: ${autoModel.label}` : "Choose from compatible models"}
                      selected={autoSelected}
                      active={activeIndex === 0}
                      showProvider
                      onClick={pickAuto}
                    />
                  ) : null}
                  {pinned.length > 0 ? <ModelSectionLabel>PINNED</ModelSectionLabel> : null}
                  {pinned.map((model) => (
                    <ModelOption key={model.id} model={model} selected={!autoSelected && model.id === selected} active={filtered[activeIndex - (autoOptionVisible ? 1 : 0)]?.id === model.id} showProvider={configuredProviders.length !== 1 || activeProvider === null} pinned onClick={() => pickModel(model)} onTogglePin={() => void toggleFavoriteModel(model.id)} />
                  ))}
                  {recent.length > 0 ? <ModelSectionLabel>RECENT</ModelSectionLabel> : null}
                  {recent.map((model) => (
                    <ModelOption key={model.id} model={model} selected={!autoSelected && model.id === selected} active={filtered[activeIndex - (autoOptionVisible ? 1 : 0)]?.id === model.id} showProvider={configuredProviders.length !== 1 || activeProvider === null} onClick={() => pickModel(model)} onTogglePin={() => void toggleFavoriteModel(model.id)} />
                  ))}
                  {showSections && (pinned.length > 0 || recent.length > 0) ? <ModelSectionLabel>ALL MODELS</ModelSectionLabel> : null}
                  {remaining.map((model) => (
                    <ModelOption key={model.id} model={model} selected={!autoSelected && model.id === selected} active={filtered[activeIndex - (autoOptionVisible ? 1 : 0)]?.id === model.id} showProvider={configuredProviders.length !== 1 || activeProvider === null} onClick={() => pickModel(model)} onTogglePin={() => void toggleFavoriteModel(model.id)} />
                  ))}
                </>
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
            className="flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-[12px] text-muted-foreground transition-colors hover:bg-foreground/[0.055]"
          >
            <HugeiconsIcon icon={Settings01Icon} size={12} strokeWidth={1.75} />
            Model settings…
          </button>
        </div>
      </PopoverContent>
    </Popover>
  );
}

function ModelSectionLabel({ children }: { children: string }) {
  return (
    <div className="px-3 pt-2 pb-1 text-[9px] font-medium tracking-[0.12em] text-muted-foreground/70">
      {children}
    </div>
  );
}

function ModelOption({
  model,
  label,
  detail,
  selected,
  active,
  showProvider,
  pinned = false,
  onClick,
  onTogglePin,
}: {
  model: ModelInfo;
  label?: string;
  detail?: string;
  selected: boolean;
  active: boolean;
  showProvider: boolean;
  pinned?: boolean;
  onClick: () => void;
  onTogglePin?: () => void;
}) {
  const Icon = PROVIDER_ICON[model.provider];
  return (
    <div className="group/model-option relative mx-1 my-0.5">
      <button
        type="button"
        id={label ? undefined : modelOptionDomId(model.id)}
        role="option"
        aria-selected={selected}
        data-active={active || undefined}
        onClick={onClick}
        className={cn(
          "flex w-full items-center gap-2 rounded-md px-2 py-1.5 pr-8 text-left",
          selected
            ? "bg-foreground/[0.085] text-popover-foreground"
            : active
              ? "bg-foreground/[0.065] text-popover-foreground"
              : "text-popover-foreground hover:bg-foreground/[0.055]",
        )}
      >
        {showProvider ? (
          <HugeiconsIcon
            icon={Icon}
            size={13}
            strokeWidth={1.5}
            className="shrink-0 text-muted-foreground/70"
          />
        ) : null}
        <span className="min-w-0 flex-1">
          <span className="block truncate text-[12px] font-medium">{label ?? model.label}</span>
          {detail ? <span className="block truncate text-[10px] text-muted-foreground">{detail}</span> : null}
        </span>
        {selected ? (
          <HugeiconsIcon icon={Tick01Icon} size={13} strokeWidth={2} className="shrink-0" />
        ) : null}
      </button>
      {onTogglePin ? (
        <button
          type="button"
          aria-label={`${pinned ? "Unpin" : "Pin"} ${model.label}`}
          title={pinned ? "Unpin model" : "Pin model"}
          onClick={(event) => {
            event.stopPropagation();
            onTogglePin();
          }}
          className={cn(
            "absolute top-1/2 right-1 -translate-y-1/2 rounded-md px-1.5 py-0.5 text-[10px] transition-colors",
            pinned
              ? "text-foreground"
              : "text-muted-foreground opacity-0 group-hover/model-option:opacity-100 hover:bg-foreground/[0.08] hover:text-foreground",
          )}
        >
          {pinned ? "Pinned" : "Pin"}
        </button>
      ) : null}
    </div>
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
          ? "bg-foreground/[0.085] text-popover-foreground after:absolute after:top-1.5 after:right-0 after:bottom-1.5 after:w-[2px] after:rounded-full after:bg-primary after:content-['']"
          : "text-muted-foreground hover:bg-foreground/[0.055]",
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
