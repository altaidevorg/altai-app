import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ComposerFollowupBar } from "../components/ComposerFollowupBar.js";

describe("ComposerFollowupBar", () => {
  it("renders hint and optional actions", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerFollowupBar, {
        hint: "Enter queues next · ⌘/Ctrl+Enter steers this run",
        showSteer: true,
        showQueue: true,
        canSteer: true,
        canQueue: false,
        onSteer: () => {},
        onQueue: () => {},
      }),
    );
    expect(html).toContain("Enter queues next");
    expect(html).toContain("Steer now");
    expect(html).toContain("Queue next");
    expect(html).toContain("disabled");
  });
});
