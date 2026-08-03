import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { Cancel01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { RowIconButton } from "../components/RowIconButton.js";

describe("RowIconButton", () => {
  it("renders title/aria-label and children", () => {
    const html = renderToStaticMarkup(
      createElement(
        RowIconButton,
        { title: "Delete", onClick: () => {} },
        createElement(HugeiconsIcon, { icon: Cancel01Icon, size: 11 }),
      ),
    );
    expect(html).toContain('title="Delete"');
    expect(html).toContain('aria-label="Delete"');
    expect(html).toContain("<svg");
    expect(html).toContain("size-5");
  });

  it("applies destructive tone styling", () => {
    const html = renderToStaticMarkup(
      createElement(
        RowIconButton,
        { title: "Delete", onClick: () => {}, tone: "destructive" },
        createElement(HugeiconsIcon, { icon: Cancel01Icon, size: 11 }),
      ),
    );
    expect(html).toContain("hover:bg-destructive/10");
    expect(html).toContain("hover:text-destructive");
  });

  it("applies default tone styling when no tone", () => {
    const html = renderToStaticMarkup(
      createElement(
        RowIconButton,
        { title: "Rename", onClick: () => {} },
        createElement(HugeiconsIcon, { icon: Cancel01Icon, size: 11 }),
      ),
    );
    expect(html).toContain("hover:bg-foreground/10");
    expect(html).toContain("hover:text-foreground");
  });
});
