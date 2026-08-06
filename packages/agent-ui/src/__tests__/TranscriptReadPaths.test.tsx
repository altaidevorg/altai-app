import { createElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { TranscriptReadPaths } from "../components/TranscriptReadPaths.js";

describe("TranscriptReadPaths", () => {
  it("renders basenames and full paths", () => {
    const html = renderToStaticMarkup(
      createElement(TranscriptReadPaths, {
        paths: ["src/foo.ts", "src/bar.ts"],
        onOpen: vi.fn(),
      }),
    );
    expect(html).toContain("foo.ts");
    expect(html).toContain("src/foo.ts");
    expect(html).toContain("bar.ts");
  });

  it("renders nothing for empty paths", () => {
    const html = renderToStaticMarkup(
      createElement(TranscriptReadPaths, { paths: [] }),
    );
    expect(html).toBe("");
  });
});
