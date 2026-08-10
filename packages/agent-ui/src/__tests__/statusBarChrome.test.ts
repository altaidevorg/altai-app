import { describe, expect, it } from "vitest";
import {
  aiAgentToggleTitle,
  voiceInputControlDisabled,
  voiceInputControlTitle,
} from "../lib/statusBarChrome.js";

describe("statusBarChrome", () => {
  it("builds agent toggle title", () => {
    expect(aiAgentToggleTitle(true, "⌘I")).toBe("Hide AI agent  ⌘I");
    expect(aiAgentToggleTitle(false, "Ctrl+I")).toBe("Show AI agent  Ctrl+I");
  });

  it("builds voice titles and disabled flag", () => {
    expect(
      voiceInputControlTitle({
        hasKey: false,
        recording: false,
        transcribing: false,
      }),
    ).toContain("OpenAI key");
    expect(
      voiceInputControlTitle({
        hasKey: true,
        recording: true,
        transcribing: false,
      }),
    ).toBe("Stop & transcribe");
    expect(
      voiceInputControlTitle({
        hasKey: true,
        recording: false,
        transcribing: true,
      }),
    ).toBe("Transcribing…");
    expect(
      voiceInputControlTitle({
        hasKey: true,
        recording: false,
        transcribing: false,
      }),
    ).toBe("Voice input");
    expect(
      voiceInputControlDisabled(false, {
        hasKey: true,
        recording: false,
        transcribing: false,
      }),
    ).toBe(false);
    expect(
      voiceInputControlDisabled(true, {
        hasKey: true,
        recording: false,
        transcribing: false,
      }),
    ).toBe(true);
  });
});
