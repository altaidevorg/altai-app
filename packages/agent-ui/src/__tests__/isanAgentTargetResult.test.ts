import { describe, expect, it } from "vitest";
import { toIsanAgentTargetResolution } from "../lib/isanAgentTargetResult.js";

const target = {
  providerName: "openai",
  apiKey: "sk",
  modelName: "gpt",
  baseUrl: "https://api",
};

describe("toIsanAgentTargetResolution", () => {
  it("returns ok target", () => {
    expect(toIsanAgentTargetResolution("gpt", target, null)).toEqual({
      ok: true,
      target,
    });
  });

  it("returns error for unresolved", () => {
    const r = toIsanAgentTargetResolution("lmstudio-local", null, null);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toMatch(/LM Studio/);
  });
});
