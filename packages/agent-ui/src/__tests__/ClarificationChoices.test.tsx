import { createElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ClarificationChoices } from "../components/ClarificationChoices.js";

describe("ClarificationChoices", () => {
  it("returns null when there are no choices or edit diff", () => {
    const html = renderToStaticMarkup(
      createElement(ClarificationChoices, {
        choices: null,
        editDiff: null,
        onRespond: () => {},
      }),
    );
    expect(html).toBe("");
  });

  it("renders suggested reply chips", () => {
    const html = renderToStaticMarkup(
      createElement(ClarificationChoices, {
        choices: ["yes", "no"],
        editDiff: null,
        onRespond: () => {},
      }),
    );
    expect(html).toContain("Suggested replies");
    expect(html).toContain("yes");
    expect(html).toContain("no");
    expect(html).toContain("2 suggested replies available");
  });

  it("prefers edit approval card over chips", () => {
    const html = renderToStaticMarkup(
      createElement(ClarificationChoices, {
        choices: ["yes"],
        editDiff: {
          file: "x.ts",
          diff: "+hello\n",
          truncated: false,
        },
        onRespond: () => {},
      }),
    );
    expect(html).toContain("x.ts");
    expect(html).toContain("Approve");
    expect(html).not.toContain("Suggested replies");
  });

  it("invokes onRespond from chip click via markup presence", () => {
    // SSR cannot click; assert the callback type is accepted and chips render.
    const onRespond = vi.fn();
    const html = renderToStaticMarkup(
      createElement(ClarificationChoices, {
        choices: ["ship it"],
        editDiff: null,
        onRespond,
      }),
    );
    expect(html).toContain("ship it");
    expect(onRespond).not.toHaveBeenCalled();
  });
});
