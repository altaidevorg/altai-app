import { describe, expect, it } from "vitest";
import {
  gitDiffContextDetailLabel,
  terminalContextDetailLabel,
} from "../lib/taskContextSourceChrome.js";

describe("terminalContextDetailLabel", () => {
  it("covers private / available / empty states", () => {
    expect(
      terminalContextDetailLabel({
        terminalPrivate: true,
        terminalAvailable: false,
      }),
    ).toMatch(/private/i);
    expect(
      terminalContextDetailLabel({
        terminalPrivate: false,
        terminalAvailable: true,
      }),
    ).toMatch(/Latest visible/i);
    expect(
      terminalContextDetailLabel({
        terminalPrivate: false,
        terminalAvailable: false,
      }),
    ).toMatch(/No terminal/i);
  });
});

describe("gitDiffContextDetailLabel", () => {
  it("depends on workspace availability", () => {
    expect(gitDiffContextDetailLabel(true)).toMatch(/unstaged/i);
    expect(gitDiffContextDetailLabel(false)).toMatch(/workspace/i);
  });
});
