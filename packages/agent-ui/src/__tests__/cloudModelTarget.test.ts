import { describe, expect, it } from "vitest";
import { resolveCloudModelTarget } from "../lib/cloudModelTarget.js";

const catalog = [
  { id: "gpt-4o", provider: "openai", apiName: "gpt-4o" },
  { id: "local", provider: "lmstudio" },
];

describe("resolveCloudModelTarget", () => {
  it("resolves with key", () => {
    expect(
      resolveCloudModelTarget("gpt-4o", catalog, {
        providerBaseUrls: { openai: "https://api.openai.com/v1/chat/completions" },
        apiKeys: { openai: "sk" },
        providerNeedsKey: () => true,
      }),
    ).toEqual({
      providerName: "openai",
      apiKey: "sk",
      modelName: "gpt-4o",
      baseUrl: "https://api.openai.com/v1/chat/completions",
    });
  });

  it("null when missing key or unknown", () => {
    expect(
      resolveCloudModelTarget("gpt-4o", catalog, {
        providerBaseUrls: { openai: "u" },
        apiKeys: { openai: "" },
        providerNeedsKey: () => true,
      }),
    ).toBeNull();
    expect(
      resolveCloudModelTarget("nope", catalog, {
        providerBaseUrls: {},
        apiKeys: {},
        providerNeedsKey: () => false,
      }),
    ).toBeNull();
  });
});
