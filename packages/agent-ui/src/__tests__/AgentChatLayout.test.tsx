import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AgentChatLayout } from "../components/AgentChatLayout.js";

describe("AgentChatLayout", () => {
  it("stacks history above main in sidebar density", () => {
    const html = renderToStaticMarkup(
      createElement(
        AgentChatLayout,
        {
          density: "sidebar",
          history: createElement("aside", null, "sessions"),
          main: createElement("main", null, "chat"),
        },
      ),
    );
    expect(html).toContain('data-density="sidebar"');
    expect(html).toContain("flex-col");
    expect(html).toContain("sessions");
    expect(html).toContain("chat");
  });

  it("places history beside main in desktop density", () => {
    const html = renderToStaticMarkup(
      createElement(
        AgentChatLayout,
        {
          density: "desktop",
          history: createElement("aside", null, "rail"),
          main: createElement("main", null, "pane"),
        },
      ),
    );
    expect(html).toContain('data-density="desktop"');
    expect(html).toContain("flex-row");
  });
});
