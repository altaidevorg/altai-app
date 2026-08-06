import { describe, expect, it } from "vitest";
import {
  hrefToFilePath,
  isWebHref,
  resolveWorkspacePath,
} from "../lib/chatHref.js";

describe("isWebHref", () => {
  it("detects http(s)/mailto/tel", () => {
    expect(isWebHref("https://example.com")).toBe(true);
    expect(isWebHref("http://example.com/path")).toBe(true);
    expect(isWebHref("mailto:a@b.com")).toBe(true);
    expect(isWebHref("tel:+123")).toBe(true);
    expect(isWebHref("/Users/me/file.ts")).toBe(false);
    expect(isWebHref("docs/readme.md")).toBe(false);
  });
});

describe("resolveWorkspacePath", () => {
  it("keeps absolute paths", () => {
    expect(resolveWorkspacePath("/abs/a.ts", "/ws")).toBe("/abs/a.ts");
  });

  it("joins with the root separator style", () => {
    expect(resolveWorkspacePath("docs/a.md", "/ws")).toBe("/ws/docs/a.md");
    expect(resolveWorkspacePath("x.ts", "C:\\ws")).toBe("C:\\ws\\x.ts");
  });

  it("throws for relatives without a root", () => {
    expect(() => resolveWorkspacePath("rel.ts", null)).toThrow(/no active workspace/);
  });
});

describe("hrefToFilePath", () => {
  it("returns absolute paths as-is", () => {
    expect(hrefToFilePath("/Users/me/a.ts", "/ws")).toBe("/Users/me/a.ts");
    expect(hrefToFilePath("C:\\Users\\me\\a.ts", null)).toBe(
      "C:\\Users\\me\\a.ts",
    );
  });

  it("resolves relative paths against workspace root", () => {
    expect(hrefToFilePath("docs/a.md", "/ws")).toBe("/ws/docs/a.md");
    expect(hrefToFilePath("./src/x.ts", "/ws")).toBe("/ws/src/x.ts");
  });

  it("returns null for web urls and unresolved relatives", () => {
    expect(hrefToFilePath("https://x.com", "/ws")).toBeNull();
    expect(hrefToFilePath("docs/a.md", null)).toBeNull();
    expect(hrefToFilePath("streamdown:incomplete-link", "/ws")).toBeNull();
  });

  it("parses file:// urls", () => {
    expect(hrefToFilePath("file:///Users/me/a.ts", null)).toBe(
      "/Users/me/a.ts",
    );
  });

  it("strips Windows drive slash for file:// paths", () => {
    expect(hrefToFilePath("file:///C:/Users/me/a.ts", null)).toBe(
      "C:/Users/me/a.ts",
    );
  });
});
