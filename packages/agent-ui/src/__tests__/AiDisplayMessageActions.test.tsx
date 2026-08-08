import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiDisplayMessageActions } from "../components/AiDisplayMessageActions.js";

describe("AiDisplayMessageActions", () => {
  it("renders only flagged slots", () => {
    const html = renderToStaticMarkup(
      createElement(AiDisplayMessageActions, {
        flags: {
          showCopy: true,
          showEdit: false,
          showRetry: true,
          showOpenFile: false,
          showOpenDiff: false,
        },
        copy: createElement("button", null, "Copy"),
        edit: createElement("button", null, "Edit"),
        retry: createElement("button", null, "Retry"),
      }),
    );
    expect(html).toContain("Copy");
    expect(html).toContain("Retry");
    expect(html).not.toContain("Edit");
  });

  it("returns empty when no flags", () => {
    const html = renderToStaticMarkup(
      createElement(AiDisplayMessageActions, {
        flags: {
          showCopy: false,
          showEdit: false,
          showRetry: false,
          showOpenFile: false,
          showOpenDiff: false,
        },
        copy: createElement("button", null, "Copy"),
      }),
    );
    expect(html).toBe("");
  });
});
