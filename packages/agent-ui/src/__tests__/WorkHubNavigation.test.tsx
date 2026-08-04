import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { WorkHubNavigation } from "../components/WorkHubNavigation.js";

describe("WorkHubNavigation", () => {
  it("renders runs and scheduled tabs", () => {
    const html = renderToStaticMarkup(
      createElement(WorkHubNavigation, {
        view: "runs",
        onViewChange: () => {},
      }),
    );
    expect(html).toContain("Work view");
    expect(html).toContain("Runs");
    expect(html).toContain("Scheduled");
  });

  it("marks scheduled as selected", () => {
    const html = renderToStaticMarkup(
      createElement(WorkHubNavigation, {
        view: "scheduled",
        onViewChange: () => {},
      }),
    );
    expect(html).toContain('aria-selected="true"');
    expect(html).toContain("Scheduled");
  });
});
