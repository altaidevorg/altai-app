import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ModelSectionLabel } from "../components/ModelSectionLabel.js";

describe("ModelSectionLabel", () => {
  it("renders children with section label styling", () => {
    const html = renderToStaticMarkup(
      createElement(ModelSectionLabel, null, "PINNED"),
    );
    expect(html).toContain("PINNED");
    expect(html).toContain("tracking-[0.12em]");
    expect(html).toContain("text-muted-foreground/70");
  });

  it("renders arbitrary node children", () => {
    const html = renderToStaticMarkup(
      createElement(
        ModelSectionLabel,
        null,
        createElement("span", null, "ALL MODELS"),
      ),
    );
    expect(html).toContain("ALL MODELS");
  });
});
