import { describe, expect, it } from "vitest";
import { useDisplayMessageInteractionState } from "../hooks/useDisplayMessageInteractionState.js";

describe("useDisplayMessageInteractionState", () => {
  it("exports a hook function", () => {
    expect(typeof useDisplayMessageInteractionState).toBe("function");
  });
});
