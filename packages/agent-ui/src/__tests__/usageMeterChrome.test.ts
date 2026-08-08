import { describe, expect, it } from "vitest";
import {
  accumulateRunUsage,
  formatRunTokenLabel,
  formatTokenCount,
  usageDeltaFromPayload,
  ZERO_RUN_USAGE,
} from "../lib/usageMeterChrome.js";

describe("usageDeltaFromPayload", () => {
  it("reads snake_case stdio usage payloads", () => {
    expect(
      usageDeltaFromPayload({
        type: "usage",
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
        cache_read_tokens: 10,
      }),
    ).toEqual({
      promptTokens: 100,
      completionTokens: 50,
      cacheReadTokens: 10,
      totalTokens: 150,
    });
  });

  it("rejects non-usage event types", () => {
    expect(
      usageDeltaFromPayload({ type: "message", prompt_tokens: 1 }),
    ).toBeNull();
  });
});

describe("accumulate + labels", () => {
  it("sums samples", () => {
    const once = accumulateRunUsage(ZERO_RUN_USAGE, {
      promptTokens: 100,
      completionTokens: 20,
      cacheReadTokens: 5,
      totalTokens: 120,
    });
    const twice = accumulateRunUsage(once, {
      promptTokens: 10,
      completionTokens: 5,
      cacheReadTokens: 0,
      totalTokens: 15,
    });
    expect(twice).toEqual({
      inputTokens: 110,
      outputTokens: 25,
      cachedInputTokens: 5,
      totalTokens: 135,
    });
  });

  it("formats compact counts and prefers totals", () => {
    expect(formatTokenCount(999)).toBe("999");
    expect(formatTokenCount(1200)).toBe("1.2k");
    expect(
      formatRunTokenLabel({
        hasActiveRun: true,
        status: "running",
        usage: {
          inputTokens: 100,
          outputTokens: 50,
          cachedInputTokens: 0,
          totalTokens: 150,
        },
      }),
    ).toBe("Tokens · 150");
    expect(
      formatRunTokenLabel({
        hasActiveRun: true,
        status: "running",
        usage: ZERO_RUN_USAGE,
      }),
    ).toBe("Tokens · live");
  });
});
