import { describe, expect, it } from "vitest";
import { parseComposerSlashLead } from "../lib/parseComposerSlashLead.js";

describe("parseComposerSlashLead", () => {
  it("returns null for plain text or empty head", () => {
    expect(parseComposerSlashLead("hello")).toBeNull();
    expect(parseComposerSlashLead("/")).toBeNull();
    expect(parseComposerSlashLead("   ")).toBeNull();
  });

  it("parses lead head and tail", () => {
    expect(parseComposerSlashLead("/fix please")).toEqual({
      lead: "/",
      head: "fix",
      tail: "please",
    });
    expect(parseComposerSlashLead("#agents coder")).toEqual({
      lead: "#",
      head: "agents",
      tail: "coder",
    });
    expect(parseComposerSlashLead("  /status  ")).toEqual({
      lead: "/",
      head: "status",
      tail: "",
    });
  });
});
