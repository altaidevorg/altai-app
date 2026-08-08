import { describe, expect, it } from "vitest";
import {
  isStandaloneReadToolPart,
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
