import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ProviderConnectBanner } from "../components/ProviderConnectBanner.js";

describe("ProviderConnectBanner", () => {
  it("renders default copy and action", () => {
    const html = renderToStaticMarkup(
      createElement(ProviderConnectBanner, { onAdd: () => {} }),
    );
    expect(html).toContain("No provider connected");
    expect(html).toContain("Connect");
    expect(html).toContain("<svg");
    expect(html).toContain("h-8");
  });

  it("accepts custom message and label", () => {
    const html = renderToStaticMarkup(
      createElement(ProviderConnectBanner, {
        onAdd: () => {},
        message: "Add a key to continue",
        actionLabel: "Open settings",
      }),
    );
    expect(html).toContain("Add a key to continue");
    expect(html).toContain("Open settings");
  });
});
