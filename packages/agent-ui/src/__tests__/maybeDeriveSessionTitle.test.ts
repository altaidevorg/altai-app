import { describe, expect, it } from "vitest";
import { DEFAULT_SESSION_TITLE } from "../lib/backendSessionTitle.js";
import { maybeDeriveSessionTitleList } from "../lib/maybeDeriveSessionTitle.js";

describe("maybeDeriveSessionTitleList", () => {
  it("renames untitled only when changed", () => {
    const sessions = [
      { id: "a", title: DEFAULT_SESSION_TITLE, updatedAt: 1 },
    ];
    expect(maybeDeriveSessionTitleList(sessions, "a", "Hello", 2)?.[0]).toMatchObject({
      title: "Hello",
      updatedAt: 2,
    });
    expect(maybeDeriveSessionTitleList(sessions, "a", DEFAULT_SESSION_TITLE)).toBeNull();
    expect(
      maybeDeriveSessionTitleList(
        [{ id: "a", title: "Fixed", updatedAt: 1 }],
        "a",
        "Hello",
      ),
    ).toBeNull();
  });
});
