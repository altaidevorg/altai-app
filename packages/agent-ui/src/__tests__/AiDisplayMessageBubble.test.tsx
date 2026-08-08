import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  AiDisplayMessageBubble,
  displayMessageElementId,
} from "../components/AiDisplayMessageBubble.js";

describe("AiDisplayMessageBubble", () => {
  it("renders role label body and id", () => {
    const html = renderToStaticMarkup(
      createElement(AiDisplayMessageBubble, {
        messageId: "m1",
        role: "user",
        body: createElement("p", null, "hello"),
      }),
    );
    expect(html).toContain(displayMessageElementId("m1"));
    expect(html).toContain("You");
    expect(html).toContain("hello");
    expect(html).toContain("altai-chat-bubble--user");
  });

  it("shows edit slot while isEditing and hides actions", () => {
    const html = renderToStaticMarkup(
      createElement(AiDisplayMessageBubble, {
        messageId: "m2",
        role: "assistant",
        isEditing: true,
        editSlot: createElement("textarea", null),
        body: createElement("p", null, "hidden body"),
        actions: createElement("button", null, "Copy"),
      }),
    );
    expect(html).toContain("textarea");
    expect(html).not.toContain("hidden body");
    expect(html).not.toContain("Copy");
  });
});
