import { describe, expect, it } from "vitest";
import { parseSkillInstallSource } from "./skillInstallSource";

describe("parseSkillInstallSource", () => {
  it("parses owner/repo", () => {
    expect(parseSkillInstallSource("altaidevorg/skills")).toEqual({
      repo: "altaidevorg/skills",
    });
  });

  it("parses hash skill filter", () => {
    expect(parseSkillInstallSource("altaidevorg/skills#review")).toEqual({
      repo: "altaidevorg/skills",
      skill: "review",
    });
  });

  it("parses space skill filter", () => {
    expect(parseSkillInstallSource("altaidevorg/skills review-pr")).toEqual({
      repo: "altaidevorg/skills",
      skill: "review-pr",
    });
  });

  it("keeps full URLs intact", () => {
    expect(
      parseSkillInstallSource("https://github.com/altaidevorg/skills#review"),
    ).toEqual({
      repo: "https://github.com/altaidevorg/skills",
      skill: "review",
    });
  });
});
