import { ChatGptIcon, ClaudeIcon } from "@hugeicons/core-free-icons";
import { createElement, createRef } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ModelPickerPanel } from "../components/ModelPickerPanel.js";

describe("ModelPickerPanel", () => {
  it("renders search, sections, and settings footer", () => {
    const html = renderToStaticMarkup(
      createElement(ModelPickerPanel, {
        search: "",
        onSearchChange: () => {},
        searchInputRef: createRef<HTMLInputElement>(),
        providers: [
          { id: "openai", label: "OpenAI", icon: ChatGptIcon },
          { id: "anthropic", label: "Anthropic", icon: ClaudeIcon },
        ],
        activeProviderId: null,
        onSelectProvider: () => {},
        pinned: [{ id: "gpt-4o", label: "GPT-4o", providerIcon: ChatGptIcon }],
        recent: [],
        remaining: [
          { id: "claude", label: "Claude", providerIcon: ClaudeIcon },
        ],
        showSections: true,
        selectedId: "gpt-4o",
        autoSelected: false,
        activeId: "gpt-4o",
        showProvider: true,
        optionDomId: (id) => `model-option-${id}`,
        onPick: () => {},
        onTogglePin: () => {},
        onOpenSettings: () => {},
      }),
    );
    expect(html).toContain("Search models");
    expect(html).toContain("PINNED");
    expect(html).toContain("ALL MODELS");
    expect(html).toContain("GPT-4o");
    expect(html).toContain("Claude");
    expect(html).toContain("Model settings");
    expect(html).toContain("All providers");
  });

  it("shows empty message and auto option", () => {
    const empty = renderToStaticMarkup(
      createElement(ModelPickerPanel, {
        search: "zzz",
        onSearchChange: () => {},
        providers: [],
        activeProviderId: null,
        onSelectProvider: () => {},
        emptyMessage: "No models match.",
        pinned: [],
        recent: [],
        remaining: [],
        showSections: false,
        selectedId: null,
        autoSelected: false,
        activeId: undefined,
        showProvider: true,
        optionDomId: (id) => id,
        onPick: () => {},
        onTogglePin: () => {},
        onOpenSettings: () => {},
      }),
    );
    expect(empty).toContain("No models match.");

    const withAuto = renderToStaticMarkup(
      createElement(ModelPickerPanel, {
        search: "",
        onSearchChange: () => {},
        providers: [],
        activeProviderId: null,
        onSelectProvider: () => {},
        autoOption: {
          modelLabel: "GPT-4o",
          providerIcon: ChatGptIcon,
          domId: "auto",
          detail: "Recommended now: GPT-4o",
          selected: true,
          active: true,
          onClick: () => {},
        },
        pinned: [],
        recent: [],
        remaining: [
          { id: "gpt-4o", label: "GPT-4o", providerIcon: ChatGptIcon },
        ],
        showSections: true,
        selectedId: "gpt-4o",
        autoSelected: true,
        activeId: "gpt-4o",
        showProvider: true,
        optionDomId: (id) => id,
        onPick: () => {},
        onTogglePin: () => {},
        onOpenSettings: () => {},
      }),
    );
    expect(withAuto).toContain("Auto");
    expect(withAuto).toContain("Recommended now: GPT-4o");
  });
});
