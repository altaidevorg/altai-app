import { describe, expect, it } from "vitest";
import {
  isStandaloneReadToolPart,
  resolveAssistantRunActionMode,
  shouldShowAssistantRunActions,
} from "../lib/chatSdkAssistantChrome.js";

describe("chatSdkAssistantChrome", () => {
  it("shows run actions while streaming or retryable", () => {
    expect(shouldShowAssistantRunActions({ streaming: true })).toBe(true);
    expect(
      shouldShowAssistantRunActions({ streaming: false, canRetry: true }),
    ).toBe(true);
    expect(shouldShowAssistantRunActions({ streaming: false })).toBe(false);
  });

  it("resolves stop, retry, or hidden mode", () => {
    expect(resolveAssistantRunActionMode({ streaming: true })).toBe("stop");
    expect(
      resolveAssistantRunActionMode({ streaming: true, canRetry: true }),
    ).toBe("stop");
    expect(
      resolveAssistantRunActionMode({ streaming: false, canRetry: true }),
    ).toBe("retry");
    expect(resolveAssistantRunActionMode({ streaming: false })).toBe("hidden");
  });

  it("detects standalone read rows", () => {
    expect(
      isStandaloneReadToolPart({
        type: "tool-read_file",
        state: "output-available",
      }),
    ).toBe(true);
    expect(
      isStandaloneReadToolPart({
        type: "tool-read_file",
        state: "approval-requested",
      }),
    ).toBe(false);
    expect(
      isStandaloneReadToolPart({ type: "tool-exec", state: "output-available" }),
    ).toBe(false);
  });
});
