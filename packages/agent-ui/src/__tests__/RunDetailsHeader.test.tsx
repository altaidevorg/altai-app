import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { RunDetailsHeader } from "../components/RunDetailsHeader.js";

describe("RunDetailsHeader", () => {
  it("renders idle status without stop", () => {
    const html = renderToStaticMarkup(
      createElement(RunDetailsHeader, {
        subtitle: "Ready for the next task",
        status: "idle",
      }),
    );
    expect(html).toContain("Run details");
    expect(html).toContain("Idle");
    expect(html).not.toContain("Stop run");
  });

  it("renders running status with stop", () => {
    const html = renderToStaticMarkup(
      createElement(RunDetailsHeader, {
        subtitle: "Editing files",
        status: "running",
        onStop: () => {},
      }),
    );
    expect(html).toContain("Running");
    expect(html).toContain("Stop run");
  });
});
