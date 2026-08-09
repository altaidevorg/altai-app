import { describe, expect, it } from "vitest";
import { resolveConfiguredLocalTargetCandidate } from "../lib/configuredLocalTarget.js";

const candidates = [
  {
    catalogId: "lmstudio-local",
    providerName: "lmstudio",
    modelName: "local-model",
    baseUrl: "http://localhost:1234/v1",
    apiKey: "",
  },
  {
    catalogId: "openai-compatible-custom",
    providerName: "openai-compatible",
    modelName: "custom-x",
    baseUrl: "https://example.com/v1",
    apiKey: "k",
  },
] as const;

describe("resolveConfiguredLocalTargetCandidate", () => {
  it("matches catalog id and appends chat path", () => {
    expect(
      resolveConfiguredLocalTargetCandidate("lmstudio-local", candidates),
    ).toEqual({
      providerName: "lmstudio",
      apiKey: "",
      modelName: "local-model",
      baseUrl: "http://localhost:1234/v1/chat/completions",
    });
  });

  it("matches raw configured model id", () => {
    expect(
      resolveConfiguredLocalTargetCandidate("custom-x", candidates),
    ).toMatchObject({ modelName: "custom-x", apiKey: "k" });
  });

  it("returns null for unknown or incomplete", () => {
    expect(
      resolveConfiguredLocalTargetCandidate("nope", candidates),
    ).toBeNull();
    expect(
      resolveConfiguredLocalTargetCandidate("lmstudio-local", [
        {
          catalogId: "lmstudio-local",
          providerName: "lmstudio",
          modelName: "",
          baseUrl: "http://x/v1",
          apiKey: "",
        },
      ]),
    ).toBeNull();
  });
});
