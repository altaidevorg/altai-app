import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ConversationOwnerSection } from "../components/ConversationOwnerSection.js";

describe("ConversationOwnerSection", () => {
  it("renders header, run-in label, picker, and children", () => {
    const html = renderToStaticMarkup(
      createElement(ConversationOwnerSection, {
        picker: createElement("button", { type: "button" }, "Select a chat"),
        children: createElement("div", null, "Footer"),
      }),
    );
    expect(html).toContain("Conversation");
    expect(html).toContain("Run in");
    expect(html).toContain("Select a chat");
    expect(html).toContain("Footer");
  });
});
