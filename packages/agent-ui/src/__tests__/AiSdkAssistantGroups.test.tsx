/** @jsxImportSource react */
import { describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiSdkAssistantGroups } from "../components/AiSdkAssistantGroups.js";
import { buildTranscriptPartGroups } from "../lib/transcriptToolGroups.js";

describe("AiSdkAssistantGroups", () => {
  it("collapses consecutive reads and renders singles via host", () => {
    const parts = [
      {
        type: "tool-read_file",
        state: "output-available",
        toolCallId: "r1",
        input: { path: "a.ts" },
      },
      {
        type: "tool-read_file",
        state: "output-available",
        toolCallId: "r2",
        input: { path: "b.ts" },
      },
      { type: "text", toolCallId: "t", text: "done" } as {
        type: string;
        text: string;
      },
    ];
    const groups = buildTranscriptPartGroups(parts);
    const renderPart = vi.fn(
      ({ part }: { part: unknown }) =>
        createElement(
          "span",
          { "data-part": (part as { type?: string }).type },
          (part as { text?: string }).text ?? "part",
        ),
    );

    const html = renderToStaticMarkup(
      createElement(AiSdkAssistantGroups, {
        messageId: "m1",
        groups,
        streaming: false,
        lastTextPartIdx: 2,
        onApproval: () => undefined,
        onOpenPath: () => undefined,
        renderPart,
        icons: {
          file: createElement("span", { "data-icon": "f" }),
          web: createElement("span", { "data-icon": "w" }),
          terminal: createElement("span", { "data-icon": "c" }),
        },
      }),
    );

    expect(html).toContain("Read");
    expect(html).toContain("file");
    expect(html).toContain('data-part="text"');
    expect(renderPart).toHaveBeenCalled();
  });
});
