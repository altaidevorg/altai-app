import { describe, expect, it } from "vitest";
import { createMessageId } from "../lib/messageId.js";

describe("createMessageId", () => {
  it("prefixes", () => {
    expect(createMessageId("req")).toMatch(/^req-/);
  });
});
