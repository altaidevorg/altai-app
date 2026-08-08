import { describe, expect, it } from "vitest";
import { displayToolGroupIconKey } from "../lib/displayToolGroupIcon.js";

describe("displayToolGroupIconKey", () => {
  it("maps group kinds", () => {
    expect(displayToolGroupIconKey("reads")).toBe("file");
    expect(displayToolGroupIconKey("cmd")).toBe("terminal");
    expect(displayToolGroupIconKey("web")).toBe("globe");
  });
});
