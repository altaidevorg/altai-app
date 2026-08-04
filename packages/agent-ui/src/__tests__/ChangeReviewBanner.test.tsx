import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ChangeReviewBanner } from "../components/ChangeReviewBanner.js";

describe("ChangeReviewBanner", () => {
  it("renders nothing when queue is empty", () => {
    const html = renderToStaticMarkup(
      createElement(ChangeReviewBanner, { queueLen: 0, onOpen: () => {} }),
    );
    expect(html).toBe("");
  });

  it("renders singular copy for one change", () => {
    const html = renderToStaticMarkup(
      createElement(ChangeReviewBanner, { queueLen: 1, onOpen: () => {} }),
    );
    expect(html).toContain("Changes ready");
    expect(html).toContain("1 proposed change waiting for");
    expect(html).toContain("Review changes");
  });

  it("renders plural copy for multiple changes", () => {
    const html = renderToStaticMarkup(
      createElement(ChangeReviewBanner, { queueLen: 3, onOpen: () => {} }),
    );
    expect(html).toContain("3 proposed changes waiting for");
  });
});
