/** @jsxImportSource react */
import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiUserTurnBody } from "../components/AiUserTurnBody.js";

describe("AiUserTurnBody", () => {
  it("renders command, chips, and text", () => {
    const html = renderToStaticMarkup(
      createElement(AiUserTurnBody, {
        commandName: "fix",
        chips: [{ kind: "file", name: "a.ts", lines: 0 }],
        text: "hello",
      }),
    );
    expect(html).toContain("/fix");
    expect(html).toContain("a.ts");
    expect(html).toContain("hello");
  });

  it("returns null when empty", () => {
    const html = renderToStaticMarkup(createElement(AiUserTurnBody, {}));
    expect(html).toBe("");
  });

  it("accepts a custom text slot", () => {
    const html = renderToStaticMarkup(
      createElement(AiUserTurnBody, {
        textSlot: createElement("div", { "data-custom": "1" }, "md"),
      }),
    );
    expect(html).toContain('data-custom="1"');
    expect(html).toContain("md");
  });
});
