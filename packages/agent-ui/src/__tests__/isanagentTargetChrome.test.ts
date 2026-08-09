import { describe, expect, it } from "vitest";
import {
  describeUnresolvedIsanAgentTarget,
  isConfiguredLocalCatalogId,
  toChatCompletionsUrl,
} from "../lib/isanagentTargetChrome.js";

describe("toChatCompletionsUrl", () => {
  it("returns empty for blank", () => {
    expect(toChatCompletionsUrl("")).toBe("");
    expect(toChatCompletionsUrl("   ")).toBe("");
  });

  it("appends chat/completions to SDK root", () => {
    expect(toChatCompletionsUrl("http://localhost:1234/v1")).toBe(
      "http://localhost:1234/v1/chat/completions",
    );
    expect(toChatCompletionsUrl("http://localhost:1234/v1/")).toBe(
      "http://localhost:1234/v1/chat/completions",
    );
  });

  it("leaves full chat paths alone", () => {
    expect(
      toChatCompletionsUrl("http://x/v1/chat/completions"),
    ).toBe("http://x/v1/chat/completions");
    expect(toChatCompletionsUrl("https://api.anthropic.com/v1/messages")).toBe(
      "https://api.anthropic.com/v1/messages",
    );
  });
});

describe("isConfiguredLocalCatalogId", () => {
  it("recognizes local catalog ids", () => {
    expect(isConfiguredLocalCatalogId("lmstudio-local")).toBe(true);
    expect(isConfiguredLocalCatalogId("gpt-4o")).toBe(false);
  });
});

describe("describeUnresolvedIsanAgentTarget", () => {
  it("special-cases local catalog misconfig", () => {
    expect(describeUnresolvedIsanAgentTarget("lmstudio-local", null)).toMatch(
      /LM Studio/,
    );
    expect(
      describeUnresolvedIsanAgentTarget("openai-compatible-custom", null),
    ).toMatch(/OpenAI-compatible/);
  });

  it("reports missing key vs unknown model", () => {
    expect(describeUnresolvedIsanAgentTarget("x", "openai")).toMatch(
      /No API key set for openai/,
    );
    expect(describeUnresolvedIsanAgentTarget("nope", null)).toBe(
      "Unknown model: nope",
    );
  });
});
