import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { RunActionRequiredSection } from "../components/RunActionRequiredSection.js";

describe("RunActionRequiredSection", () => {
  it("renders title and children", () => {
    const html = renderToStaticMarkup(
      createElement(RunActionRequiredSection, {
        children: createElement("div", null, "Approvals"),
      }),
    );
    expect(html).toContain("Action required");
    expect(html).toContain("Approvals");
  });
});
