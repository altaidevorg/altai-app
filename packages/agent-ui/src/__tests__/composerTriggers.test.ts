import { describe, expect, it } from "vitest";
import {
  detectAtMention,
  detectSlashOrSnippetTrigger,
  nextAtMentionIndex,
  removeAtMentionToken,
  shouldSearchAtMention,
} from "../lib/composerTriggers.js";

describe("detectAtMention", () => {
  it("finds @query under the caret", () => {
    const text = "look at @src/foo";
    const cursor = text.length;
    expect(detectAtMention(text, cursor)).toEqual({
      start: 8,
      end: cursor,
      query: "src/foo",
    });
  });

  it("requires @ after boundary", () => {
    expect(detectAtMention("email@host", 10)).toBeNull();
    expect(detectAtMention("hi @x", 5)).toEqual({
      start: 3,
      end: 5,
      query: "x",
    });
  });

  it("returns null outside a mention", () => {
    expect(detectAtMention("hello world", 5)).toBeNull();
  });
});

describe("detectSlashOrSnippetTrigger", () => {
  it("detects leading slash commands", () => {
    expect(detectSlashOrSnippetTrigger("/tasks x", 6)).toEqual({
      start: 0,
      end: 6,
      query: "tasks",
      prefix: "/",
    });
  });

  it("rejects mid-message slash", () => {
    expect(detectSlashOrSnippetTrigger("please /tasks", 13)).toBeNull();
  });

  it("detects # snippets", () => {
    expect(detectSlashOrSnippetTrigger("use #foo", 8)).toEqual({
      start: 4,
      end: 8,
      query: "foo",
      prefix: "#",
    });
  });
});

describe("removeAtMentionToken", () => {
  it("strips the open token", () => {
    expect(
      removeAtMentionToken("see @pack then", {
        start: 4,
        end: 9,
        query: "pack",
      }),
    ).toBe("see then");
  });
});

describe("shouldSearchAtMention", () => {
  it("requires non-empty query", () => {
    expect(shouldSearchAtMention("")).toBe(false);
    expect(shouldSearchAtMention("a")).toBe(true);
  });
});

describe("nextAtMentionIndex", () => {
  it("navigates and picks", () => {
    expect(nextAtMentionIndex("ArrowDown", 0, 3).activeIndex).toBe(1);
    expect(nextAtMentionIndex("ArrowUp", 0, 3).activeIndex).toBe(0);
    expect(nextAtMentionIndex("Enter", 2, 3).pick).toBe(true);
    expect(nextAtMentionIndex("Escape", 1, 3).dismiss).toBe(true);
  });
});
