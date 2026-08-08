import { describe, expect, it } from "vitest";
import { canMountReplayControl } from "../lib/replayChrome.js";

describe("canMountReplayControl", () => {
  it("requires capability, chat, and run id", () => {
    expect(
      canMountReplayControl({
        replay: true,
        chatId: "c1",
        runId: "r1",
      }),
    ).toBe(true);
    expect(
      canMountReplayControl({
        replay: false,
        chatId: "c1",
        runId: "r1",
      }),
    ).toBe(false);
    expect(
      canMountReplayControl({
        replay: true,
        chatId: null,
        runId: "r1",
      }),
    ).toBe(false);
  });
});
