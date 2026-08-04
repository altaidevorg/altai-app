import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ComposerConfigRow } from "../components/ComposerConfigRow.js";

describe("ComposerConfigRow", () => {
  it("renders model-only and agent+model layouts", () => {
    const modelOnly = renderToStaticMarkup(
      createElement(ComposerConfigRow, {
        modelSlot: createElement("button", null, "Model"),
      }),
    );
    expect(modelOnly).toContain("Chat configuration");
    expect(modelOnly).toContain("grid-cols-1");
    expect(modelOnly).toContain("Model");

    const both = renderToStaticMarkup(
      createElement(ComposerConfigRow, {
        agentSlot: createElement("button", null, "Agent"),
        modelSlot: createElement("button", null, "Model"),
      }),
    );
    expect(both).toContain("grid-cols-2");
    expect(both).toContain("Agent");
  });
});
