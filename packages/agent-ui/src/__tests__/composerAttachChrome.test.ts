import { describe, expect, it } from "vitest";
import {
  buildDiffContextItem,
  buildFileContextItem,
  buildSelectionContextItem,
  buildTerminalContextItem,
} from "../lib/composerAttachChrome.js";

describe("composerAttachChrome builders", () => {
  it("builds a file chip from uri/path", () => {
    const item = buildFileContextItem({
      uri: "file:///ws/a.ts",
      path: "/ws/a.ts",
    });
    expect(item?.kind).toBe("file");
    expect(item?.name).toBe("a.ts");
  });

  it("builds selection and rejects empty text", () => {
    expect(buildSelectionContextItem({ uri: "", path: "", text: "  " })).toBeNull();
    const item = buildSelectionContextItem({
      uri: "file:///ws/a.ts",
      path: "/ws/a.ts",
      text: "const x = 1;",
    });
    expect(item?.kind).toBe("selection");
    expect(item?.lines).toBe(1);
  });

  it("builds diff from files summary", () => {
    const item = buildDiffContextItem({
      branch: "main",
      files: [{ path: "a.ts", status: "M" }],
    });
    expect(item?.kind).toBe("diff");
    expect(item?.text).toContain("a.ts");
  });

  it("builds terminal from selection preference", () => {
    const item = buildTerminalContextItem({
      selectedText: "npm test",
      lastCommand: "echo hi",
      cwd: "/ws",
    });
    expect(item?.kind).toBe("terminal");
    expect(item?.text).toBe("npm test");
  });
});
