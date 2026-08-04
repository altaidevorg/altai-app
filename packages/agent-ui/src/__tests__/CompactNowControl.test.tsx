import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { CompactNowControl } from "../components/CompactNowControl.js";

describe("CompactNowControl", () => {
  it("renders the compact affordance", () => {
    const html = renderToStaticMarkup(
      createElement(CompactNowControl, {
        onClick: () => {},
      }),
    );
    expect(html).toContain("Compact context (run /compact now)");
    expect(html).toContain("<svg");
    expect(html).toContain("<button");
  });

  it("can be disabled", () => {
    const html = renderToStaticMarkup(
      createElement(CompactNowControl, {
        disabled: true,
        onClick: () => {},
      }),
    );
    expect(html).toContain("disabled");
  });
});
