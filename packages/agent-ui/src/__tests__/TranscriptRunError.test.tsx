import { createElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { TranscriptRunError } from "../components/TranscriptRunError.js";

describe("TranscriptRunError", () => {
  it("renders fatal error chrome", () => {
    const html = renderToStaticMarkup(
      createElement(TranscriptRunError, {
        message: "provider timeout",
        onDismiss: vi.fn(),
      }),
    );
    expect(html).toContain('role="alert"');
    expect(html).toContain("Something went wrong.");
    expect(html).toContain("provider timeout");
    expect(html).toContain("Dismiss");
  });

  it("renders attention variant title", () => {
    const html = renderToStaticMarkup(
      createElement(TranscriptRunError, {
        message: "Run paused — budget",
        variant: "attention",
      }),
    );
    expect(html).toContain("Run needs attention");
    expect(html).toContain("border-warning/40");
  });
});
