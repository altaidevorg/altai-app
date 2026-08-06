import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AiBookIcon } from "@hugeicons/core-free-icons";
import {
  AuxiliarySurface,
  SurfaceHeader,
  SurfaceTabs,
} from "../components/AuxiliarySurface.js";

describe("AuxiliarySurface", () => {
  it("renders SurfaceHeader title and close control", () => {
    const html = renderToStaticMarkup(
      createElement(SurfaceHeader, {
        title: "Inbox",
        icon: AiBookIcon,
        onClose: () => {},
      }),
    );
    expect(html).toContain("Inbox");
    expect(html).toContain('aria-label="Close Inbox"');
  });

  it("renders SurfaceTabs with selection", () => {
    const html = renderToStaticMarkup(
      createElement(SurfaceTabs, {
        label: "Work sections",
        value: "runs",
        onChange: () => {},
        items: [
          { id: "runs", label: "Runs", count: 2 },
          { id: "scheduled", label: "Scheduled" },
        ],
      }),
    );
    expect(html).toContain("Runs");
    expect(html).toContain('aria-selected="true"');
  });

  it("uses embedded presentation without absolute overlay chrome", () => {
    const html = renderToStaticMarkup(
      createElement(AuxiliarySurface, {
        title: "Work",
        presentation: "embedded",
        children: createElement("div", null, "body"),
      }),
    );
    expect(html).toContain("Work");
    expect(html).toContain("body");
    expect(html).toContain("h-full");
    expect(html).not.toContain("absolute inset-0");
  });
});
