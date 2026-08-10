import { describe, expect, it } from "vitest";
import {
  AUTOMATIONS_INSTRUCTION_TITLE,
  AUTOMATIONS_MESSAGE_PLACEHOLDER,
} from "../lib/automationsFormChrome.js";

describe("automationsFormChrome", () => {
  it("exposes create form field chrome", () => {
    expect(AUTOMATIONS_INSTRUCTION_TITLE).toBe("Instruction");
    expect(AUTOMATIONS_MESSAGE_PLACEHOLDER).toContain("agent");
  });
});
