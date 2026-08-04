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
  Settings01Icon,
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
import { ComposerConfigTrigger, ModelPickerPanel } from "@altai/agent-ui";

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
        <ModelPickerPanel
          search={search}
          onSearchChange={setSearch}
          onSearchKeyDown={handleInputKeyDown}
          searchInputRef={inputRef}
          listboxId={MODEL_LISTBOX_ID}
          activeDescendantId={
            filtered[activeIndex - (autoOptionVisible ? 1 : 0)]
              ? modelOptionDomId(
                  filtered[activeIndex - (autoOptionVisible ? 1 : 0)].id,
                )
              : undefined
          }
          providers={configuredProviders.map((provider) => ({
            id: provider.id,
            label: provider.label,
            icon: PROVIDER_ICON[provider.id],
          }))}
          activeProviderId={activeProvider}
          onSelectProvider={(id) =>
            setActiveProvider(id as ProviderId | null)
          }
          emptyMessage={
            filtered.length === 0
              ? available.length === 0
                ? "No models available — add an API key in Model settings."
                : (describeModelConstraint(activeAgent) ?? "No models match.")
              : null
          }
          autoOption={
            autoOptionVisible
              ? {
                  modelLabel: autoModel?.label ?? current.label,
                  providerIcon:
                    PROVIDER_ICON[autoModel?.provider ?? current.provider],
                  domId: modelOptionDomId(autoModel?.id ?? current.id),
                  detail: autoModel
                    ? `Recommended now: ${autoModel.label}`
                    : "Choose from compatible models",
                  selected: autoSelected,
                  active: activeIndex === 0,
                  onClick: pickAuto,
                }
              : null
          }
          pinned={pinned.map((model) => ({
            id: model.id,
            label: model.label,
            providerIcon: PROVIDER_ICON[model.provider],
          }))}
          recent={recent.map((model) => ({
            id: model.id,
            label: model.label,
            providerIcon: PROVIDER_ICON[model.provider],
          }))}
          remaining={remaining.map((model) => ({
            id: model.id,
            label: model.label,
            providerIcon: PROVIDER_ICON[model.provider],
          }))}
          showSections={showSections}
          selectedId={selected}
          autoSelected={autoSelected}
          activeId={filtered[activeIndex - (autoOptionVisible ? 1 : 0)]?.id}
          showProvider={
            configuredProviders.length !== 1 || activeProvider === null
          }
          optionDomId={modelOptionDomId}
          onPick={(id) => {
            const model = filtered.find((entry) => entry.id === id);
            if (model) pickModel(model);
          }}
          onTogglePin={(id) => {
            void toggleFavoriteModel(id);
          }}
          onOpenSettings={() => {
            setOpen(false);
            void openSettingsWindow("models");
          }}
        />
      </PopoverContent>
    </Popover>
  );
}

/** @deprecated kept so callers that checked key presence still compile */
export function modelNeedsKey(provider: ProviderId): boolean {
  return providerNeedsKey(provider);
}
