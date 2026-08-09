import { describe, expect, it } from "vitest";
import { fallbackSpecFromTarget } from "../lib/fallbackSpec.js";

describe("fallbackSpecFromTarget", () => {
  const target = {
    providerName: "openai",
    baseUrl: "https://api.openai.com/v1/chat/completions",
    apiKey: "sk",
    modelName: "gpt-4o",
  };

  it("returns null for empty id or unresolved target", () => {
    expect(fallbackSpecFromTarget("", target)).toBeNull();
    expect(fallbackSpecFromTarget("gpt-4o", null)).toBeNull();
  });

  it("maps fields", () => {
    expect(fallbackSpecFromTarget("gpt-4o", target)).toEqual(target);
  });
});
