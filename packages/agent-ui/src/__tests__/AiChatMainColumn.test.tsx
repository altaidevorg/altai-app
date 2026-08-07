import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AiChatMainColumn } from "../components/AiChatMainColumn.js";

describe("AiChatMainColumn", () => {
  it("renders slots in Desktop density order", () => {
    const html = renderToStaticMarkup(
      createElement(AiChatMainColumn, {
        planMode: createElement("div", { "data-slot": "plan" }, "Plan"),
        transcript: createElement("div", { "data-slot": "tx" }, "Messages"),
        runChrome: createElement("div", { "data-slot": "run" }, "Run"),
        composer: createElement("div", { "data-slot": "composer" }, "Composer"),
        footer: createElement("div", { "data-slot": "footer" }, "Footer"),
      }),
    );
    expect(html).toContain("altai-ai-chat-main");
    expect(html).toContain('id="altai-active-chat"');
    expect(html).toContain('role="tabpanel"');
    expect(html).toContain("altai-ai-chat-transcript");
    const plan = html.indexOf('data-slot="plan"');
    const tx = html.indexOf('data-slot="tx"');
    const run = html.indexOf('data-slot="run"');
    const composer = html.indexOf('data-slot="composer"');
    const footer = html.indexOf('data-slot="footer"');
    expect(plan).toBeLessThan(tx);
    expect(tx).toBeLessThan(run);
    expect(run).toBeLessThan(composer);
    expect(composer).toBeLessThan(footer);
  });

  it("omits optional slots when unused", () => {
    const html = renderToStaticMarkup(
      createElement(AiChatMainColumn, {
        transcript: "Empty",
        composer: "Composer",
      }),
    );
    expect(html).toContain("Empty");
    expect(html).toContain("Composer");
    expect(html).not.toContain("data-slot");
  });
});
