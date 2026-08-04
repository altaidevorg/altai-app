import { createElement, type ReactElement, type ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ComposerToolbarIcon } from "../components/ComposerToolbarIcon.js";

describe("ComposerToolbarIcon", () => {
  it("renders an accessible icon button", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerToolbarIcon, {
        title: "Attach file",
        onClick: () => {},
        children: createElement("span", null, "📎"),
      }),
    );
    expect(html).toContain('aria-label="Attach file"');
    expect(html).toContain("📎");
    expect(html).toContain("<button");
  });

  it("uses renderTooltip and respects disabled", () => {
    const wrap = vi.fn(
      (label: string, children: ReactElement): ReactNode =>
        createElement("div", { "data-tip": label }, children),
    );
    const html = renderToStaticMarkup(
      createElement(ComposerToolbarIcon, {
        title: "Dictate",
        disabled: true,
        onClick: () => {},
        renderTooltip: wrap,
        children: createElement("span", null, "mic"),
      }),
    );
    expect(wrap).toHaveBeenCalledOnce();
    expect(html).toContain('data-tip="Dictate"');
    expect(html).toContain("disabled");
  });
});
