import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { SettingsTab } from "@/modules/settings/openSettingsWindow";
import { usePreferencesStore } from "@/modules/settings/preferences";
import {
  AiScanIcon,
  CodeSquareIcon,
  GithubIcon,
  InformationCircleIcon,
  Layers02Icon,
  PuzzleIcon,
  PlugIcon,
  Settings01Icon,
  UniversalAccessIcon,
  UserMultiple02Icon,
  KeyboardIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { JSX, useEffect } from "react";
import { AboutSection } from "./sections/AboutSection";
import { AccessibilitySection } from "./sections/AccessibilitySection";
import { AgentsSection } from "./sections/AgentsSection";
import { ContextSection } from "./sections/ContextSection";
import { GeneralSection } from "./sections/GeneralSection";
import { GitHubSection } from "./sections/GitHubSection";
import { HooksSection } from "./sections/HooksSection";
import { LanguageServersSection } from "./sections/LanguageServersSection";
import { McpSection } from "./sections/McpSection";
import { ModelsSection } from "./sections/ModelsSection";
import { ShortcutsSection } from "./sections/ShortcutsSection";
import { SkillsSection } from "./sections/SkillsSection";

export type SettingsSurface = "app" | "ide";

const TABS: {
  id: SettingsTab;
  label: string;
  icon: typeof Settings01Icon;
  component: () => JSX.Element;
  surfaces: SettingsSurface[];
}[] = [
  // Desktop Agents (app) vs Desktop IDE (ide) catalogs stay intentionally split.
  // The VS Code plugin uses its own hub — never this table.
  { id: "general", label: "General", icon: Settings01Icon, component: () => <GeneralSection />, surfaces: ["app", "ide"] },
  { id: "shortcuts", label: "Shortcuts", icon: KeyboardIcon, component: ShortcutsSection, surfaces: ["ide"] },
  { id: "models", label: "Models", icon: AiScanIcon, component: ModelsSection, surfaces: ["app", "ide"] },
  { id: "context", label: "Context", icon: Layers02Icon, component: ContextSection, surfaces: ["app", "ide"] },
  { id: "agents", label: "Agents", icon: UserMultiple02Icon, component: AgentsSection, surfaces: ["app", "ide"] },
  { id: "skills", label: "Skills", icon: PuzzleIcon, component: SkillsSection, surfaces: ["app", "ide"] },
  { id: "github", label: "GitHub", icon: GithubIcon, component: GitHubSection, surfaces: ["app"] },
  { id: "language-servers", label: "Languages", icon: CodeSquareIcon, component: LanguageServersSection, surfaces: ["ide"] },
  { id: "mcp", label: "MCP", icon: PlugIcon, component: McpSection, surfaces: ["app", "ide"] },
  { id: "hooks", label: "Hooks", icon: CodeSquareIcon, component: HooksSection, surfaces: ["app", "ide"] },
  { id: "accessibility", label: "Accessibility", icon: UniversalAccessIcon, component: AccessibilitySection, surfaces: ["app", "ide"] },
  { id: "about", label: "About", icon: InformationCircleIcon, component: () => <AboutSection />, surfaces: ["app", "ide"] },
];

export const VALID_SETTINGS_TABS: SettingsTab[] = TABS.map((t) => t.id);

/** Normalize legacy / unknown section ids. */
export function normalizeSettingsTab(
  input: string | undefined,
  surface?: SettingsSurface,
): SettingsTab {
  if (input === "ai" || input === "connections") return "models";
  if (input === "plugins" || input === "marketplace") return "general";
  if (input === "compaction" || input === "isanagentignore") return "context";
  // "project" moved to the Operations sidebar; redirect to context
  // for any persisted/legacy references to the old settings tab.
  if (input === "project") return "context";
  if (input && (VALID_SETTINGS_TABS as string[]).includes(input)) {
    const tab = input as SettingsTab;
    if (
      !surface ||
      TABS.find((item) => item.id === tab)?.surfaces.includes(surface)
    ) {
      return tab;
    }
  }
  return "general";
}

/**
 * Reusable settings shell shared by the two products. The navigation and
 * available sections are surface-specific so Studio preferences cannot leak
 * into IDE preferences (or vice versa).
 *
 * The active section is fully controlled — the host owns it so it can
 * persist across re-mounts (e.g. survive tab focus/unfocus).
 */
export function SettingsContent({
  active,
  onActiveChange,
  surface = "app",
}: {
  active: SettingsTab;
  onActiveChange: (next: SettingsTab) => void;
  surface?: SettingsSurface;
}) {
  const init = usePreferencesStore((s) => s.init);
  const visibleTabs = TABS.filter((tab) => tab.surfaces.includes(surface));
  const normalizedActive = normalizeSettingsTab(active, surface);
  const ActiveSection = visibleTabs.find((t) => t.id === normalizedActive)?.component;
  const isApp = surface === "app";

  useEffect(() => {
    void init();
  }, [init]);

  if (!isApp) {
    return (
      <div className="flex h-full min-h-0 flex-col overflow-hidden">
        <div className="flex h-11 shrink-0 items-center gap-3 border-b border-border/60 bg-card/30 px-3">
          <div className="hidden shrink-0 sm:block">
            <div className="text-[11px] font-semibold leading-none">
              Desktop IDE
            </div>
            <div className="mt-0.5 text-[10px] text-muted-foreground">
              Editor + agent
            </div>
          </div>
          <Tabs
            value={normalizedActive}
            onValueChange={(v) => onActiveChange(v as SettingsTab)}
            orientation="horizontal"
            className="flex min-w-0 flex-1 items-center"
          >
            <TabsList className="mx-auto h-7 max-w-full overflow-x-auto bg-muted/40 px-2 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
              {visibleTabs.map((t) => (
                <TabsTrigger
                  key={t.id}
                  value={t.id}
                  className="h-6 gap-1.5 px-2.5 text-[11.5px]"
                >
                  <HugeiconsIcon icon={t.icon} size={12} strokeWidth={1.75} />
                  <span>{t.label}</span>
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>
        </div>

        <main className="min-h-0 flex-1 overflow-y-auto px-8 pt-6 pb-7 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
          <div className="mx-auto w-full max-w-160">
            {normalizedActive === "general" ? (
              <GeneralSection surface="ide" />
            ) : normalizedActive === "about" ? (
              <AboutSection surface="ide" />
            ) : ActiveSection ? (
              <ActiveSection />
            ) : null}
          </div>
        </main>
      </div>
    );
  }

  return (
    <Tabs
      value={normalizedActive}
      onValueChange={(v) => onActiveChange(v as SettingsTab)}
      orientation="vertical"
      className="flex h-full min-h-0 flex-row gap-0 overflow-hidden"
    >
      <aside className="flex w-48 shrink-0 flex-col border-r border-border/60 bg-card/30 px-2.5 py-4">
        <div className="mb-4 flex items-center gap-2.5 px-2">
          <span className="flex size-8 shrink-0 items-center justify-center rounded-xl border border-border/60 bg-background">
            <HugeiconsIcon
              icon={AiScanIcon}
              size={16}
              strokeWidth={1.7}
            />
          </span>
          <div className="min-w-0">
            <div className="truncate text-[12px] font-semibold">
              ALTAI Desktop
            </div>
            <div className="text-[10.5px] text-muted-foreground">
              Agents settings
            </div>
          </div>
        </div>

        <TabsList
          variant="line"
          className="h-auto w-full flex-col items-stretch justify-start gap-0.5 rounded-none bg-transparent p-0"
        >
            {visibleTabs.map((t) => (
              <TabsTrigger
                key={t.id}
                value={t.id}
                className="h-8 w-full flex-none justify-start gap-2 rounded-lg px-2.5 text-[11.5px] font-medium data-active:bg-muted/70 after:hidden"
              >
                <HugeiconsIcon icon={t.icon} size={14} strokeWidth={1.7} />
                <span>{t.label}</span>
              </TabsTrigger>
            ))}
        </TabsList>

        <p className="mt-auto px-2 pt-4 text-[10px] leading-4 text-muted-foreground/70">
          Desktop Agents preferences — separate from Desktop IDE and the VS Code
          extension.
        </p>
      </aside>

      <main className="min-h-0 flex-1 overflow-y-auto px-8 pt-6 pb-7 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
        <div className="mx-auto w-full max-w-160">
          {normalizedActive === "general" ? (
            <GeneralSection surface={surface} />
          ) : normalizedActive === "about" ? (
            <AboutSection surface={surface} />
          ) : ActiveSection ? (
            <ActiveSection />
          ) : null}
        </div>
      </main>
    </Tabs>
  );
}
