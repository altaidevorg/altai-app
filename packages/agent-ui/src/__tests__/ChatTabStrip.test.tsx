import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ChatTabStrip } from "../components/ChatTabStrip.js";

describe("ChatTabStrip", () => {
  it("renders tabs, active selection, and new-chat control", () => {
    const html = renderToStaticMarkup(
      createElement(ChatTabStrip, {
        tabs: [
          { id: "a", title: "First" },
          { id: "b", title: "" },
        ],
        activeId: "a",
        onSelect: () => {},
        onClose: () => {},
        onNewChat: () => {},
      }),
    );
    expect(html).toContain('aria-label="Open chats"');
    expect(html).toContain("First");
    expect(html).toContain("New chat");
    expect(html).toContain('aria-selected="true"');
    expect(html).toContain("Close First");
    expect(html).toContain("Close new chat");
    expect(html).toContain("<svg");
  });

  it("uses host tooltip wrapper when provided", () => {
    const html = renderToStaticMarkup(
      createElement(ChatTabStrip, {
        tabs: [{ id: "a", title: "Chat" }],
        activeId: "a",
        onSelect: () => {},
        onClose: () => {},
        onNewChat: () => {},
        renderTooltip: (label, children) =>
          createElement(
            "div",
            { "data-tooltip": label },
            children,
          ),
      }),
    );
    expect(html).toContain('data-tooltip="Close Chat"');
    expect(html).toContain('data-tooltip="New chat"');
  });
});
