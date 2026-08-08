import { describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiDisplayMessageEditForm } from "../components/AiDisplayMessageEditForm.js";

describe("AiDisplayMessageEditForm", () => {
  it("renders cancel and save chrome", () => {
    const html = renderToStaticMarkup(
      createElement(AiDisplayMessageEditForm, {
        value: "hello",
        onChange: vi.fn(),
        onCancel: vi.fn(),
        onSave: vi.fn(),
      }),
    );
    expect(html).toContain("hello");
    expect(html).toContain("Cancel");
    expect(html).toContain("Save &amp; resend");
  });

  it("disables save when empty", () => {
    const html = renderToStaticMarkup(
      createElement(AiDisplayMessageEditForm, {
        value: "   ",
        onChange: vi.fn(),
        onCancel: vi.fn(),
        onSave: vi.fn(),
      }),
    );
    expect(html).toContain("disabled");
  });
});
