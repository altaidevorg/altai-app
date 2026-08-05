import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SurfaceInlineError } from "../components/SurfaceInlineError.js";

describe("SurfaceInlineError", () => {
  it("renders message and dismiss control", () => {
    const html = renderToStaticMarkup(
      createElement(SurfaceInlineError, {
        message: "Schedule failed",
        onDismiss: () => {},
        dismissAriaLabel: "Dismiss automation error",
      }),
    );
    expect(html).toContain("role=\"alert\"");
    expect(html).toContain("Schedule failed");
    expect(html).toContain("Dismiss");
    expect(html).toContain("Dismiss automation error");
  });

  it("omits dismiss without callback", () => {
    const html = renderToStaticMarkup(
      createElement(SurfaceInlineError, { message: "Boom" }),
    );
    expect(html).toContain("Boom");
    expect(html).not.toContain("Dismiss");
  });
});
