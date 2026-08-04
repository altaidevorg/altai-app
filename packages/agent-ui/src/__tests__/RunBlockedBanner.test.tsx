import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { RunBlockedBanner } from "../components/RunBlockedBanner.js";

describe("RunBlockedBanner", () => {
  it("renders title and message", () => {
    const html = renderToStaticMarkup(
      createElement(RunBlockedBanner, {
        message: "Permission denied writing src/app.ts",
      }),
    );
    expect(html).toContain("Run blocked");
    expect(html).toContain("Permission denied writing src/app.ts");
  });
});
