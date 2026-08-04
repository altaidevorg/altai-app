import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { File01Icon } from "@hugeicons/core-free-icons";
import { ContextSourceToggle } from "../components/ContextSourceToggle.js";

describe("ContextSourceToggle", () => {
  it("renders label, detail, and switch role", () => {
    const html = renderToStaticMarkup(
      createElement(ContextSourceToggle, {
        icon: File01Icon,
        label: "Active file",
        detail: "Attach the open editor",
        checked: false,
        onChange: () => {},
      }),
    );
    expect(html).toContain("Active file");
    expect(html).toContain("Attach the open editor");
    expect(html).toContain('role="switch"');
    expect(html).toContain('aria-checked="false"');
    expect(html).toContain("<svg");
  });

  it("reflects checked state in aria-checked and styling", () => {
    const html = renderToStaticMarkup(
      createElement(ContextSourceToggle, {
        icon: File01Icon,
        label: "Terminal",
        detail: "Attach latest output",
        checked: true,
        onChange: () => {},
      }),
    );
    expect(html).toContain('aria-checked="true"');
    expect(html).toContain("border-primary bg-primary");
    expect(html).toContain("translate-x-3");
  });

  it("forwards disabled attribute", () => {
    const html = renderToStaticMarkup(
      createElement(ContextSourceToggle, {
        icon: File01Icon,
        label: "Git diff",
        detail: "Attach unstaged changes",
        checked: false,
        disabled: true,
        onChange: () => {},
      }),
    );
    expect(html).toContain("disabled");
    expect(html).toContain("disabled:opacity-45");
  });
});
