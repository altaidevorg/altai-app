import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AiComposer } from "../components/AiComposer.js";

describe("AiComposer", () => {
  it("renders draft, config, and primary rows", () => {
    const html = renderToStaticMarkup(
      createElement(AiComposer, {
        value: "hello",
        onChange: () => {},
        tools: createElement("button", { type: "button" }, "Attach"),
        modelSlot: createElement("span", null, "Model"),
        agentSlot: createElement("span", null, "Agent"),
        permission: createElement("span", null, "Perm"),
        submit: createElement("button", { type: "button" }, "Send"),
      }),
    );
    expect(html).toContain("altai-ai-composer");
    expect(html).toContain("hello");
    expect(html).toContain("Agent");
    expect(html).toContain("Model");
    expect(html).toContain("Send");
  });
});
