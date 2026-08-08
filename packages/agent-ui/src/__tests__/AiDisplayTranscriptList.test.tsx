/** @jsxImportSource react */
import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiDisplayTranscriptList } from "../components/AiDisplayTranscriptList.js";
import type { TranscriptDisplayMessage } from "../lib/displayTranscriptBlocks.js";

describe("AiDisplayTranscriptList", () => {
  it("renders messages and collapses consecutive tools", () => {
    const messages: TranscriptDisplayMessage[] = [
      { id: "u1", role: "user", content: "hi" },
      {
        id: "t1",
        role: "tool",
        content: "read a",
        toolName: "read_file",
        filePath: "a.ts",
      },
      {
        id: "t2",
        role: "tool",
        content: "read b",
        toolName: "read_file",
        filePath: "b.ts",
      },
      { id: "a1", role: "assistant", content: "done" },
    ];

    const html = renderToStaticMarkup(
      createElement(AiDisplayTranscriptList, {
        messages,
        announce: "polite",
        renderMessage: (m: TranscriptDisplayMessage) =>
          createElement(
            "article",
            { key: m.id, "data-id": m.id },
            m.content,
          ),
        renderGroupIcon: () => createElement("span", { "data-icon": "r" }),
      }),
    );

    expect(html).toContain('role="log"');
    expect(html).toContain('id="altai-active-chat"');
    expect(html).toContain("altai-chat-tool-group");
    expect(html).toContain('data-id="u1"');
    expect(html).toContain('data-id="a1"');
    expect(html).toContain("Read");
  });
});
