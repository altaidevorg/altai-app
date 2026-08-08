import { describe, expect, it } from "vitest";
import { buildAiChatViewRowMeta } from "../lib/aiChatViewModel.js";

describe("buildAiChatViewRowMeta", () => {
  it("marks streaming assistant and last-turn retry", () => {
    const rows = buildAiChatViewRowMeta({
      messages: [
        { id: "u1", role: "user" },
        { id: "a1", role: "assistant" },
      ],
      status: "streaming",
      retryableFailure: false,
    });
    expect(rows[1]?.streaming).toBe(true);
    expect(rows[1]?.canRetry).toBe(false);

    const retryRows = buildAiChatViewRowMeta({
      messages: [
        { id: "u1", role: "user" },
        { id: "a1", role: "assistant" },
      ],
      status: "error",
      retryableFailure: true,
    });
    expect(retryRows[1]?.canRetry).toBe(true);
    expect(retryRows[0]?.canRetry).toBe(false);
  });
});
