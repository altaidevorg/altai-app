import { describe, expect, it } from "vitest";
import { buildIssueBody } from "./items";

describe("buildIssueBody", () => {
  it("keeps a plain description unchanged when criteria are empty", () => {
    expect(buildIssueBody("  Describe the problem.  ", " \n ")).toBe(
      "Describe the problem.",
    );
  });

  it("formats acceptance criteria as an unchecked GitHub task list", () => {
    expect(
      buildIssueBody(
        "Fix the login flow.",
        "Successful login redirects home\n- Invalid credentials stay visible",
      ),
    ).toBe(
      [
        "Fix the login flow.",
        "",
        "## Acceptance criteria",
        "",
        "- [ ] Successful login redirects home",
        "- [ ] Invalid credentials stay visible",
      ].join("\n"),
    );
  });

  it("normalizes pasted checklists and numbered lists", () => {
    expect(buildIssueBody("", "- [x] Existing item\n2. Numbered item")).toBe(
      [
        "## Acceptance criteria",
        "",
        "- [ ] Existing item",
        "- [ ] Numbered item",
      ].join("\n"),
    );
  });
});
