import { describe, expect, it } from "vitest";
import {
  hasApiKeyForModel,
  hasProviderApiKey,
} from "../lib/providerKeyChrome.js";

describe("hasProviderApiKey", () => {
  it("skips keys when provider needs none", () => {
    expect(
      hasProviderApiKey({
        provider: "local",
        apiKeys: {},
        providerNeedsKey: () => false,
      }),
    ).toBe(true);
  });
  it("requires non-empty key when needed", () => {
    expect(
      hasProviderApiKey({
        provider: "openai",
        apiKeys: { openai: "sk" },
        providerNeedsKey: () => true,
      }),
    ).toBe(true);
    expect(
      hasProviderApiKey({
        provider: "openai",
        apiKeys: { openai: "" },
        providerNeedsKey: () => true,
      }),
    ).toBe(false);
  });
});

describe("hasApiKeyForModel", () => {
  it("resolves provider then checks keys", () => {
    expect(
      hasApiKeyForModel({
        modelId: "gpt",
        apiKeys: { openai: "x" },
        providerForModel: () => "openai",
        providerNeedsKey: () => true,
      }),
    ).toBe(true);
  });
});
