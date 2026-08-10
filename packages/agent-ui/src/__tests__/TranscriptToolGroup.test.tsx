import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { TranscriptToolGroup } from "../components/TranscriptToolGroup.js";

describe("TranscriptToolGroup", () => {
  it("renders collapsed with label, count, and preview", () => {
    const html = renderToStaticMarkup(
      createElement(
        TranscriptToolGroup,
        {
          label: "Read",
          countLabel: "2 files",
          preview: "a.ts, b.ts",
          icon: createElement("span", null, "icon"),
          defaultOpen: false,
          children: createElement("ul", null, createElement("li", null, "row")),
        },
      ),
    );
    expect(html).toContain("Read");
    expect(html).toContain("2 files");
    expect(html).toContain("a.ts, b.ts");
    expect(html).toContain('aria-expanded="false"');
    expect(html).not.toContain("data-altai-tool-group-panel");
  });

  it("renders children when defaultOpen", () => {
    const html = renderToStaticMarkup(
      createElement(
        TranscriptToolGroup,
        {
          label: "Web",
          countLabel: "1 call",
          icon: createElement("span", null, "icon"),
          defaultOpen: true,
          children: createElement("div", { className: "body" }, "expanded"),
        },
      ),
    );
    expect(html).toContain('aria-expanded="true"');
    expect(html).toContain("data-altai-tool-group-panel");
    expect(html).toContain("expanded");
  });
});
