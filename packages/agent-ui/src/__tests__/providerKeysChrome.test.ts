import { describe, expect, it } from "vitest";
import { hasAnyProviderKey } from "../lib/providerKeysChrome.js";

const providers = [
  { id: "openai", supportsKey: true },
  { id: "lmstudio", supportsKey: false },
];

describe("hasAnyProviderKey", () => {
  it("is false when empty or only keyless", () => {
    expect(hasAnyProviderKey({}, providers)).toBe(false);
    expect(hasAnyProviderKey({ lmstudio: "x" }, providers)).toBe(false);
    expect(hasAnyProviderKey({ openai: null }, providers)).toBe(false);
  });

  it("is true when a key-using provider has a value", () => {
    expect(hasAnyProviderKey({ openai: "sk" }, providers)).toBe(true);
  });
});
