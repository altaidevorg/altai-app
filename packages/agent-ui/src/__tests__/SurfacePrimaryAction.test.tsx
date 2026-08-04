import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  SurfacePrimaryAction,
  SurfaceSecondaryAction,
} from "../components/SurfacePrimaryAction.js";

describe("SurfacePrimaryAction", () => {
  it("renders primary label", () => {
    const html = renderToStaticMarkup(
      createElement(SurfacePrimaryAction, {
        children: "Delegate work",
      }),
    );
    expect(html).toContain("Delegate work");
    expect(html).toContain("bg-primary");
  });

  it("renders secondary label", () => {
    const html = renderToStaticMarkup(
      createElement(SurfaceSecondaryAction, {
        children: "Queue",
      }),
    );
    expect(html).toContain("Queue");
    expect(html).toContain("bg-muted");
  });
});
