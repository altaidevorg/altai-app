import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { DesktopPrimaryNav } from "./DesktopPrimaryNav";

describe("DesktopPrimaryNav", () => {
  it("renders the M5-A destinations with one current page", () => {
    const html = renderToStaticMarkup(
      createElement(DesktopPrimaryNav, {
        activeDestination: "home",
        inboxCount: 3,
        onNavigate: () => {},
        onOpenIde: () => {},
        onOpenSettings: () => {},
      }),
    );

    expect(html).toContain('aria-label="Primary"');
    expect(html.match(/aria-current="page"/g)).toHaveLength(1);
    expect(html).toContain("Home");
    expect(html).toContain("Work");
    expect(html).toContain("Agents");
    expect(html).toContain("IDE");
    expect(html).toContain("3 items need attention");
  });
});
