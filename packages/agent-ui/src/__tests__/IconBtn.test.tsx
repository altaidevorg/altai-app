import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { Cancel01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { IconBtn } from "../components/IconBtn.js";

describe("IconBtn", () => {
  it("renders title and children", () => {
    const html = renderToStaticMarkup(
      createElement(
        IconBtn,
        { title: "Stop", onClick: () => {} },
        createElement(HugeiconsIcon, { icon: Cancel01Icon, size: 12 }),
      ),
    );
    expect(html).toContain('title="Stop"');
    expect(html).toContain("<svg");
    expect(html).toContain("size-6");
  });

  it("forwards disabled attribute", () => {
    const html = renderToStaticMarkup(
      createElement(
        IconBtn,
        { title: "Stop", onClick: () => {}, disabled: true },
        createElement(HugeiconsIcon, { icon: Cancel01Icon, size: 12 }),
      ),
    );
    expect(html).toContain("disabled");
    expect(html).toContain("disabled:opacity-40");
  });

  it("merges caller className", () => {
    const html = renderToStaticMarkup(
      createElement(
        IconBtn,
        { title: "X", onClick: () => {}, className: "ml-1" },
        createElement(HugeiconsIcon, { icon: Cancel01Icon, size: 12 }),
      ),
    );
    expect(html).toContain("ml-1");
    expect(html).toContain("size-6");
  });
});
