import { describe, expect, it } from "vitest";
import {
  normalizeProviderBaseUrl,
  providerRequiresBaseUrl,
} from "../lib/providerBaseUrl.js";

describe("providerBaseUrl", () => {
  it("accepts http(s) urls", () => {
    expect(normalizeProviderBaseUrl(" https://api.example/v1 ")).toBe(
      "https://api.example/v1",
    );
    expect(normalizeProviderBaseUrl("ftp://x")).toBeNull();
    expect(normalizeProviderBaseUrl("")).toBeNull();
  });
  it("requires base url only for openai-compatible", () => {
    expect(providerRequiresBaseUrl("openai-compatible")).toBe(true);
    expect(providerRequiresBaseUrl("anthropic")).toBe(false);
  });
});
