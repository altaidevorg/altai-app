import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { OperationsNavigationShell } from "../components/OperationsNavigationShell.js";

describe("OperationsNavigationShell", () => {
  it("renders shared routes and disables unavailable domain slices", () => {
    const html = renderToStaticMarkup(createElement(OperationsNavigationShell, { view: "overview", onViewChange: () => {}, availableViews: ["overview"] }, "body"));
    expect(html).toContain("Operations navigation");
    expect(html).toContain("Overview");
    expect(html).toContain("Work");
    expect(html).toContain("disabled=\"\"");
  });
});
