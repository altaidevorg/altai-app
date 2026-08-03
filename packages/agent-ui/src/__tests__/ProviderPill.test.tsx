import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AiBookIcon } from "@hugeicons/core-free-icons";
import { ProviderPill } from "../components/ProviderPill.js";

describe("ProviderPill", () => {
  it("renders icon and title attribute", () => {
    const html = renderToStaticMarkup(
      createElement(ProviderPill, {
        icon: AiBookIcon,
        title: "All providers",
        active: false,
        onClick: () => {},
      }),
    );
    expect(html).toContain('title="All providers"');
    expect(html).toContain("<svg");
    expect(html).toContain("size-7");
  });

  it("applies active styling when active", () => {
    const html = renderToStaticMarkup(
      createElement(ProviderPill, {
        icon: AiBookIcon,
        title: "Anthropic",
        active: true,
        onClick: () => {},
      }),
    );
    expect(html).toContain("bg-foreground/[0.085]");
    expect(html).toContain("after:bg-primary");
  });

  it("applies hover styling when inactive", () => {
    const html = renderToStaticMarkup(
      createElement(ProviderPill, {
        icon: AiBookIcon,
        title: "OpenAI",
        active: false,
        onClick: () => {},
      }),
    );
    expect(html).toContain("text-muted-foreground");
    expect(html).toContain("hover:bg-foreground/[0.055]");
  });
});
