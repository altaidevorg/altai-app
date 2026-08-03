import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  EditApprovalCard,
  parseDiffLines,
} from "../components/EditApprovalCard.js";

describe("EditApprovalCard", () => {
  it("parses unified diff markers", () => {
    const lines = parseDiffLines("--- a\n+++ b\n@@ -1 +1 @@\n-old\n+new\n context");
    expect(lines.map((l) => l.kind)).toEqual([
      "meta",
      "meta",
      "hunk",
      "del",
      "add",
      "ctx",
    ]);
  });

  it("renders file path and actions", () => {
    const html = renderToStaticMarkup(
      createElement(EditApprovalCard, {
        diff: {
          file: "src/app.ts",
          diff: "--- a\n+++ b\n+hello\n",
          truncated: true,
        },
        onRespond: () => {},
      }),
    );
    expect(html).toContain("src/app.ts");
    expect(html).toContain("truncated");
    expect(html).toContain("Approve");
    expect(html).toContain("Deny");
  });
});
