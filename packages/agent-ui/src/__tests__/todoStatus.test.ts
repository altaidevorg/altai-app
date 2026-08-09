import { describe, expect, it } from "vitest";
import { normalizeTodoStatus } from "../lib/todoStatus.js";

describe("normalizeTodoStatus", () => {
  it("maps LLM variants", () => {
    expect(normalizeTodoStatus("Done")).toBe("completed");
    expect(normalizeTodoStatus("in progress")).toBe("in_progress");
    expect(normalizeTodoStatus("WIP")).toBe("in_progress");
    expect(normalizeTodoStatus("")).toBe("pending");
    expect(normalizeTodoStatus(null)).toBe("pending");
  });
});
