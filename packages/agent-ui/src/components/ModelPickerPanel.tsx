import {
  AiBookIcon,
  Search01Icon,
  Settings01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import type { KeyboardEventHandler, RefObject } from "react";
import { ModelOption } from "./ModelOption.js";
import { ModelSectionLabel } from "./ModelSectionLabel.js";
import { ProviderPill } from "./ProviderPill.js";

export type ModelPickerRow = {
  id: string;
  label: string;
  providerIcon: IconSvgElement;
};

export type ModelPickerProvider = {
  id: string;
  label: string;
  icon: IconSvgElement;
};

export type ModelPickerAutoOption = {
  modelLabel: string;
  providerIcon: IconSvgElement;
  domId: string;
  detail: string;
  selected: boolean;
  active: boolean;
  onClick: () => void;
};

export type ModelPickerPanelProps = {
  search: string;
  onSearchChange: (value: string) => void;
  onSearchKeyDown?: KeyboardEventHandler<HTMLInputElement>;
  searchInputRef?: RefObject<HTMLInputElement | null>;
  listboxId?: string;
  activeDescendantId?: string;

  /** When length > 1, a provider rail is shown. */
  providers: ModelPickerProvider[];
  activeProviderId: string | null;
  onSelectProvider: (providerId: string | null) => void;
  allProvidersIcon?: IconSvgElement;

  /** When set, the list area shows this empty state instead of options. */
  emptyMessage?: string | null;

  autoOption?: ModelPickerAutoOption | null;
  pinned: ModelPickerRow[];
  recent: ModelPickerRow[];
  remaining: ModelPickerRow[];
  /** When true, PINNED / RECENT / ALL MODELS headings are shown. */
  showSections: boolean;

  selectedId: string | null;
  autoSelected: boolean;
  activeId: string | undefined;
  showProvider: boolean;
  optionDomId: (id: string) => string;

  onPick: (id: string) => void;
  onTogglePin: (id: string) => void;
  onOpenSettings: () => void;
};

/**
 * Model picker popover body: search, optional provider rail, sectioned options,
 * and settings footer. Host owns Popover chrome, filtering, and store writes.
 */
export function ModelPickerPanel({
  search,
  onSearchChange,
  onSearchKeyDown,
  searchInputRef,
  listboxId = "model-switcher-listbox",
  activeDescendantId,
  providers,
  activeProviderId,
  onSelectProvider,
  allProvidersIcon = AiBookIcon,
  emptyMessage = null,
  autoOption = null,
  pinned,
  recent,
  remaining,
  showSections,
  selectedId,
  autoSelected,
  activeId,
  showProvider,
  optionDomId,
  onPick,
  onTogglePin,
  onOpenSettings,
}: ModelPickerPanelProps) {
  const showProviderRail = providers.length > 1;

  return (
    <div className="altai-model-picker-panel flex flex-col gap-0 overflow-hidden">
      <div className="flex items-center gap-2 border-b border-border/70 px-3 py-2">
        <HugeiconsIcon
          icon={Search01Icon}
          size={14}
          strokeWidth={1.75}
          className="shrink-0 text-muted-foreground/70"
        />
        <input
          ref={searchInputRef}
          role="combobox"
          aria-expanded
          aria-controls={listboxId}
          aria-autocomplete="list"
          aria-activedescendant={activeDescendantId}
          aria-label="Search models"
          value={search}
          onChange={(e) => onSearchChange(e.target.value)}
          onKeyDown={onSearchKeyDown}
          placeholder="Search models…"
          className="w-full bg-transparent text-xs outline-none placeholder:text-muted-foreground/60"
        />
      </div>

      <div className="flex min-h-0">
        {showProviderRail ? (
          <div className="flex w-10 flex-col gap-0.5 border-r border-border/70 bg-muted/20 py-1.5">
            <ProviderPill
              icon={allProvidersIcon}
              title="All providers"
              active={activeProviderId === null}
              onClick={() => onSelectProvider(null)}
            />
            {providers.map((provider) => (
              <ProviderPill
                key={provider.id}
                icon={provider.icon}
                title={provider.label}
                active={activeProviderId === provider.id}
                onClick={() => onSelectProvider(provider.id)}
              />
            ))}
          </div>
        ) : null}

        <div className="max-h-[18rem] flex-1 overflow-y-auto py-1">
          <div id={listboxId} role="listbox" aria-label="Models">
            {emptyMessage ? (
              <div className="px-4 py-8 text-center text-xs text-muted-foreground/70">
                {emptyMessage}
              </div>
            ) : (
              <>
                {autoOption ? (
                  <ModelOption
                    modelLabel={autoOption.modelLabel}
                    providerIcon={autoOption.providerIcon}
                    domId={autoOption.domId}
                    label="Auto"
                    detail={autoOption.detail}
                    selected={autoOption.selected}
                    active={autoOption.active}
                    showProvider
                    onClick={autoOption.onClick}
                  />
                ) : null}
                {showSections && pinned.length > 0 ? (
                  <ModelSectionLabel>PINNED</ModelSectionLabel>
                ) : null}
                {pinned.map((model) => (
                  <ModelOption
                    key={model.id}
                    modelLabel={model.label}
                    providerIcon={model.providerIcon}
                    domId={optionDomId(model.id)}
                    selected={!autoSelected && model.id === selectedId}
                    active={activeId === model.id}
                    showProvider={showProvider}
                    pinned
                    onClick={() => onPick(model.id)}
                    onTogglePin={() => onTogglePin(model.id)}
                  />
                ))}
                {showSections && recent.length > 0 ? (
                  <ModelSectionLabel>RECENT</ModelSectionLabel>
                ) : null}
                {recent.map((model) => (
                  <ModelOption
                    key={model.id}
                    modelLabel={model.label}
                    providerIcon={model.providerIcon}
                    domId={optionDomId(model.id)}
                    selected={!autoSelected && model.id === selectedId}
                    active={activeId === model.id}
                    showProvider={showProvider}
                    onClick={() => onPick(model.id)}
                    onTogglePin={() => onTogglePin(model.id)}
                  />
                ))}
                {showSections && (pinned.length > 0 || recent.length > 0) ? (
                  <ModelSectionLabel>ALL MODELS</ModelSectionLabel>
                ) : null}
                {remaining.map((model) => (
                  <ModelOption
                    key={model.id}
                    modelLabel={model.label}
                    providerIcon={model.providerIcon}
                    domId={optionDomId(model.id)}
                    selected={!autoSelected && model.id === selectedId}
                    active={activeId === model.id}
                    showProvider={showProvider}
                    onClick={() => onPick(model.id)}
                    onTogglePin={() => onTogglePin(model.id)}
                  />
                ))}
              </>
            )}
          </div>
        </div>
      </div>

      <div className="border-t border-border/70 p-1">
        <button
          type="button"
          onClick={onOpenSettings}
          className="flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-[12px] text-muted-foreground transition-colors hover:bg-foreground/[0.055]"
        >
          <HugeiconsIcon icon={Settings01Icon} size={12} strokeWidth={1.75} />
          Model settings…
        </button>
      </div>
    </div>
  );
}
