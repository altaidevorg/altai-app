import { describe, expect, it } from "vitest";
import { classifySdkUiPart, sdkPartText } from "../lib/sdkUiPartKind.js";

describe("sdkUiPartKind", () => {
  it("classifies part types", () => {
    expect(classifySdkUiPart({ type: "text", text: "hi" })).toBe("text");
    expect(classifySdkUiPart({ type: "reasoning", text: "…" })).toBe(
      "reasoning",
    );
    expect(classifySdkUiPart({ type: "tool-exec" })).toBe("tool");
    expect(classifySdkUiPart({ type: "file" })).toBe("unknown");
  });

  it("reads text safely", () => {
    expect(sdkPartText({ type: "text", text: "ab" })).toBe("ab");
    expect(sdkPartText({ type: "text" })).toBe("");
  });
});
