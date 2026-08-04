import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AiBookIcon, Tick01Icon } from "@hugeicons/core-free-icons";
import { ModelOption } from "../components/ModelOption.js";

describe("ModelOption", () => {
  const baseProps = {
    modelLabel: "Claude Sonnet 4",
    selected: false,
    active: false,
    showProvider: true,
    providerIcon: AiBookIcon,
    onClick: () => {},
  };

  it("renders label and provider icon", () => {
    const html = renderToStaticMarkup(
      createElement(ModelOption, baseProps),
    );
    expect(html).toContain("Claude Sonnet 4");
    expect(html).toContain("<svg");
    expect(html).toContain('role="option"');
    expect(html).toContain('aria-selected="false"');
  });

  it("shows checkmark when selected", () => {
    const html = renderToStaticMarkup(
      createElement(ModelOption, { ...baseProps, selected: true }),
    );
    expect(html).toContain('aria-selected="true"');
    expect(html).toContain("bg-foreground/[0.085]");
  });

  it("renders pin toggle when onTogglePin provided", () => {
    const html = renderToStaticMarkup(
      createElement(ModelOption, {
        ...baseProps,
        onTogglePin: () => {},
        pinned: true,
      }),
    );
    expect(html).toContain("Pinned");
    expect(html).toContain('aria-label="Unpin Claude Sonnet 4"');
  });

  it("renders Pin button when not pinned", () => {
    const html = renderToStaticMarkup(
      createElement(ModelOption, {
        ...baseProps,
        onTogglePin: () => {},
        pinned: false,
      }),
    );
    expect(html).toContain("Pin");
    expect(html).toContain('aria-label="Pin Claude Sonnet 4"');
  });

  it("hides provider icon when showProvider is false", () => {
    const html = renderToStaticMarkup(
      createElement(ModelOption, { ...baseProps, showProvider: false }),
    );
    expect(html).not.toContain("text-muted-foreground/70");
  });
});
