import { ToolbarIconButton } from "@/components/altai";
import { WindowControls } from "@/components/WindowControls";
import { cn } from "@/lib/utils";
import { IS_MAC, KEY_SEP, USE_CUSTOM_WINDOW_CONTROLS } from "@/lib/platform";
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  getBindingTokens,
  SHORTCUTS,
  type ShortcutId,
} from "@/modules/shortcuts/shortcuts";
import type { Tab } from "@/modules/tabs";
import { TabBar } from "@/modules/tabs";
import {
  KeyboardIcon,
  Settings01Icon,
  SidebarLeftIcon,
  SidebarRightIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import {
  SearchInline,
  type SearchInlineHandle,
  type SearchTarget,
} from "./SearchInline";

type Props = {
  tabs: Tab[];
  activeId: number;
  onSelect: (id: number) => void;
  onNew: () => void;
  onNewPrivate: () => void;
  onNewPreview: () => void;
  onNewEditor: () => void;
  onNewGitGraph: () => void;
  onClose: (id: number) => void;
  /** Promote a preview (transient) tab to persistent. */
  onPin: (id: number) => void;
  onToggleSidebar: () => void;
  sidebarActive?: boolean;
  onOpenShortcuts: () => void;
  onOpenSettings: () => void;
  /** Focus the separate agent-first app window without changing IDE state. */
  onOpenAgentWorkspace?: () => void;
  onToggleAgentSidebar?: () => void;
  agentSidebarActive?: boolean;
  agentSidebarAvailable?: boolean;
  searchTarget: SearchTarget;
  searchRef: RefObject<SearchInlineHandle | null>;
  /** True when another app-level titlebar already owns the native chrome row. */
  embedded?: boolean;
};

const COMPACT_WIDTH = 720;

/** Titlebar chrome hit-target — matches the macOS traffic-light visual row
 * (header is h-10; lights are inset to the same vertical center via
 * `trafficLightPosition` in tauri.conf). Compact 24px buttons with 13px icons
 * keep the chrome visually proportional to the ~12px native traffic lights
 * while still meeting the 24px WCAG 2.5.8 minimum hit area. */
const CHROME_BTN = "size-6 translate-y-[0.5px]";
const CHROME_ICON = 12;

export function Header({
  tabs,
  activeId,
  onSelect,
  onNew,
  onNewPrivate,
  onNewPreview,
  onNewEditor,
  onNewGitGraph,
  onClose,
  onPin,
  onToggleSidebar,
  sidebarActive,
  onOpenShortcuts,
  onOpenSettings,
  onOpenAgentWorkspace,
  onToggleAgentSidebar,
  agentSidebarActive,
  agentSidebarAvailable,
  searchTarget,
  searchRef,
  embedded = false,
}: Props) {
  const rootRef = useRef<HTMLDivElement>(null);
  const [compact, setCompact] = useState(false);
  const userShortcuts = usePreferencesStore((s) => s.shortcuts);

  const tokensFor = (id: ShortcutId): string => {
    const s = SHORTCUTS.find((s) => s.id === id);
    if (!s) return "";
    const bindings = userShortcuts[id] || s.defaultBindings;
    if (!bindings || bindings.length === 0) return "";
    return getBindingTokens(bindings[0]).join(KEY_SEP);
  };

  const shortcutLabel = useMemo(() => {
    const tokens = tokensFor("shortcuts.open");
    return tokens ? `Keyboard shortcuts (${tokens})` : "Keyboard shortcuts";
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [userShortcuts]);

  useEffect(() => {
    const el = rootRef.current;
    if (!el) return;
    const ro = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width ?? 0;
      setCompact(w < COMPACT_WIDTH);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const shortcutsButton = (
    <ToolbarIconButton
      className={CHROME_BTN}
      onClick={onOpenShortcuts}
      title={shortcutLabel}
      aria-label={shortcutLabel}
    >
      <HugeiconsIcon
        icon={KeyboardIcon}
        size={CHROME_ICON}
        strokeWidth={1.75}
      />
    </ToolbarIconButton>
  );

  const settingsButton = (
    <ToolbarIconButton
      className={CHROME_BTN}
      onClick={onOpenSettings}
      title="Settings"
      aria-label="Settings"
    >
      <HugeiconsIcon
        icon={Settings01Icon}
        size={CHROME_ICON}
        strokeWidth={1.75}
      />
    </ToolbarIconButton>
  );

  const agentSidebarButton =
    agentSidebarAvailable && onToggleAgentSidebar ? (
      <ToolbarIconButton
        active={agentSidebarActive}
        className={cn(
          CHROME_BTN,
          agentSidebarActive &&
            "text-primary hover:text-primary hover:bg-primary/12 dark:hover:bg-primary/20",
        )}
        onClick={onToggleAgentSidebar}
        title={agentSidebarActive ? "Hide AI agent" : "Show AI agent"}
        aria-pressed={agentSidebarActive}
        aria-label={agentSidebarActive ? "Hide AI agent" : "Show AI agent"}
      >
        <HugeiconsIcon
          icon={SidebarRightIcon}
          size={CHROME_ICON}
          strokeWidth={1.75}
        />
      </ToolbarIconButton>
    ) : null;

  const agentWorkspaceButton = onOpenAgentWorkspace ? (
    <button
      type="button"
      onClick={onOpenAgentWorkspace}
      title="Open Agent workspace"
      aria-label="Open Agent workspace"
      className="inline-flex h-7 shrink-0 items-center gap-1.5 rounded-lg border border-border/70 bg-muted/45 px-2.5 text-[10.5px] font-medium text-foreground transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
    >
      <span>Agent workspace</span>
      <span aria-hidden="true" className="text-[12px] leading-none text-muted-foreground">
        ↗
      </span>
    </button>
  ) : null;

  return (
    <header
      ref={rootRef}
      role="banner"
      aria-label="Workspace toolbar"
      className={`flex h-10 shrink-0 items-center gap-1.5 border-b border-border-subtle bg-raised select-none ${
        IS_MAC && !embedded ? "pr-2 pl-20" : "pr-0 pl-2"
      }`}
    >
      <div className="flex shrink-0 items-center gap-0.5">
        <ToolbarIconButton
          active={sidebarActive}
          onClick={onToggleSidebar}
          title="Show or hide sidebar"
          className={cn(
            CHROME_BTN,
            sidebarActive &&
              "text-primary hover:text-primary hover:bg-primary/12 dark:hover:bg-primary/20",
          )}
          aria-label="Show or hide sidebar"
        >
          <HugeiconsIcon
            icon={SidebarLeftIcon}
            size={CHROME_ICON}
            strokeWidth={1.75}
          />
        </ToolbarIconButton>

        {!IS_MAC && shortcutsButton}
      </div>

      {!IS_MAC && <span className="mx-1 h-5 w-px shrink-0 bg-border" />}

      {IS_MAC && <span className="mr-1 h-full w-px shrink-0 bg-border" />}

      <div className="flex min-w-0 flex-1 items-center gap-2">
        <TabBar
          tabs={tabs}
          activeId={activeId}
          onSelect={onSelect}
          onNew={onNew}
          onNewPrivate={onNewPrivate}
          onNewPreview={onNewPreview}
          onNewEditor={onNewEditor}
          onNewGitGraph={onNewGitGraph}
          onClose={onClose}
          onPin={onPin}
          compact={compact}
        />
        <div data-tauri-drag-region className="h-full min-w-2 flex-1" />
      </div>

      <SearchInline ref={searchRef} target={searchTarget} compact={compact} />

      {agentWorkspaceButton}

      {IS_MAC && (
        <>
          {shortcutsButton}
          {agentSidebarButton}
          {settingsButton}
        </>
      )}

      {!IS_MAC && (
        <>
          {agentSidebarButton}
          {settingsButton}
        </>
      )}

      {USE_CUSTOM_WINDOW_CONTROLS && (
        <>
          <span className="ml-1 h-5 w-px shrink-0 bg-border" />
          <WindowControls />
        </>
      )}
    </header>
  );
}
