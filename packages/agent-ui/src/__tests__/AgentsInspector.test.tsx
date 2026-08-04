import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AgentsInspector } from "../components/AgentsInspector.js";

describe("AgentsInspector", () => {
  it("renders empty state when no tasks", () => {
    const html = renderToStaticMarkup(
      createElement(AgentsInspector, { tasks: [] }),
    );
    expect(html).toContain(
      "Delegated research, review, and test tasks will stay visible here.",
    );
  });

  it("renders task display name and child chat id", () => {
    const html = renderToStaticMarkup(
      createElement(AgentsInspector, {
        tasks: [
          {
            taskId: "t1",
            displayName: "Researcher",
            agentName: "research",
            childChatId: "chat-abc",
          },
        ],
      }),
    );
    expect(html).toContain("Researcher");
    expect(html).toContain("chat-abc");
    expect(html).toContain("animate-pulse");
  });

  it("falls back to agentName then Subagent", () => {
    const withAgent = renderToStaticMarkup(
      createElement(AgentsInspector, {
        tasks: [
          {
            taskId: "t2",
            displayName: null,
            agentName: "tester",
            childChatId: "c2",
          },
        ],
      }),
    );
    expect(withAgent).toContain("tester");

    const fallback = renderToStaticMarkup(
      createElement(AgentsInspector, {
        tasks: [
          {
            taskId: "t3",
            displayName: null,
            agentName: null,
            childChatId: "c3",
          },
        ],
      }),
    );
    expect(fallback).toContain("Subagent");
  });
});
