import { describe, expect, it } from "vitest";
import {
  clampPanelWidth,
  parsePanelWidth,
  serializePanelWidth,
} from "../lib/sidePanelWidth.js";

describe("sidePanelWidth", () => {
  it("clamps and parses", () => {
    expect(clampPanelWidth(100, 176, 360)).toBe(176);
    expect(parsePanelWidth("240", 200, 176, 360)).toBe(240);
    expect(parsePanelWidth("x", 200, 176, 360)).toBe(200);
  });
  it("serializes rounded finite widths", () => {
    expect(serializePanelWidth(250.6, 176, 360)).toBe("251");
    expect(serializePanelWidth(0, 176, 360)).toBeNull();
  });
});
