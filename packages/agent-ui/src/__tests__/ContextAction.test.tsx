import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { CodeIcon } from "@hugeicons/core-free-icons";
import { ContextAction } from "../components/ContextAction.js";

describe("ContextAction", () => {
  it("renders label, detail, and icon", () => {
    const html = renderToStaticMarkup(
      createElement(ContextAction, {
        icon: CodeIcon,
        label: "Working tree diff",
        detail: "Attach unstaged Git changes",
        disabled: false,
        onClick: () => {},
      }),
    );
    expect(html).toContain("Working tree diff");
    expect(html).toContain("Attach unstaged Git changes");
    expect(html).toContain("<svg");
  });

  it("forwards disabled attribute", () => {
    const html = renderToStaticMarkup(
      createElement(ContextAction, {
        icon: CodeIcon,
        label: "Active file",
        detail: "Attach the file open in the editor",
        disabled: true,
        onClick: () => {},
      }),
    );
    expect(html).toContain("disabled");
    expect(html).toContain("disabled:opacity-40");
  });
});
