import { describe, expect, it } from "vitest";
import {
  canMountSkillsStatus,
  skillsSummaryCopy,
  sortSkillsForDisplay,
} from "../lib/skillsStatusChrome.js";

describe("skillsStatusChrome", () => {
  it("gates on skills.list capability", () => {
    expect(canMountSkillsStatus({ skillsList: true })).toBe(true);
    expect(canMountSkillsStatus({ skillsList: false })).toBe(false);
  });

  it("sorts and summarizes", () => {
    const skills = sortSkillsForDisplay([
      { name: "zeta", enabled: false },
      { name: "alpha", enabled: true },
    ]);
    expect(skills.map((s) => s.name)).toEqual(["alpha", "zeta"]);
    expect(skillsSummaryCopy(skills)).toBe("1/2 skills enabled");
    expect(skillsSummaryCopy([])).toBe("No skills");
  });
});
