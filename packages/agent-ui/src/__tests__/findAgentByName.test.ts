import { describe, expect, it } from "vitest";
import { findAgentByIdOrName } from "../lib/findAgentByName.js";

const agents = [
  { id: "builtin:coder", name: "Coder" },
  { id: "custom:alice", name: "Alice Bot" },
];

describe("findAgentByIdOrName", () => {
  it("matches id and name case-insensitively", () => {
    expect(findAgentByIdOrName(agents, "BUILTIN:CODER")?.name).toBe("Coder");
    expect(findAgentByIdOrName(agents, "alice bot")?.id).toBe("custom:alice");
  });

  it("returns undefined for empty/miss", () => {
    expect(findAgentByIdOrName(agents, "  ")).toBeUndefined();
    expect(findAgentByIdOrName(agents, "nope")).toBeUndefined();
  });
});
