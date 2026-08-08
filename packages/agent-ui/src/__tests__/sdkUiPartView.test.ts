import { describe, expect, it } from "vitest";
import { mapSdkUiPartView } from "../lib/sdkUiPartView.js";

describe("mapSdkUiPartView", () => {
  it("maps text and reasoning with safe text", () => {
    expect(mapSdkUiPartView({ type: "text", text: "hi" })).toEqual({
      kind: "text",
      text: "hi",
    });
    expect(mapSdkUiPartView({ type: "reasoning" })).toEqual({
      kind: "reasoning",
      text: "",
    });
  });

  it("maps tool parts and unknown", () => {
    const tool = mapSdkUiPartView({
      type: "tool-exec",
      state: "output-available",
      input: { path: "a" },
    });
    expect(tool.kind).toBe("tool");
    if (tool.kind === "tool") {
      expect(tool.part.type).toBe("tool-exec");
    }
    expect(mapSdkUiPartView({ type: "file" })).toEqual({ kind: "unknown" });
  });
});
