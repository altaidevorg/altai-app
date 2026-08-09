import { describe, expect, it } from "vitest";
import { cutThroughNthUserMessage } from "../lib/cutThroughNthUser.js";

describe("cutThroughNthUserMessage", () => {
  const msgs = [
    { role: "user", id: "1" },
    { role: "assistant", id: "2" },
    { role: "user", id: "3" },
    { role: "assistant", id: "4" },
  ];
  it("keeps through Nth user", () => {
    expect(cutThroughNthUserMessage(msgs, 1).map((m) => m.id)).toEqual(["1"]);
    expect(cutThroughNthUserMessage(msgs, 2).map((m) => m.id)).toEqual([
      "1",
      "2",
      "3",
    ]);
  });
  it("noop when fewer users", () => {
    expect(cutThroughNthUserMessage(msgs, 9)).toHaveLength(4);
  });
  it("empty when keep <= 0", () => {
    expect(cutThroughNthUserMessage(msgs, 0)).toEqual([]);
  });
});
