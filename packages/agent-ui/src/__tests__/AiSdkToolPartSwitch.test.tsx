import { describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiSdkToolPartSwitch } from "../components/AiSdkToolPartSwitch.js";

describe("AiSdkToolPartSwitch", () => {
  it("routes approval-requested to approval slot", () => {
    const renderCard = vi.fn(() => null);
    const html = renderToStaticMarkup(
      createElement(AiSdkToolPartSwitch, {
        part: {
          type: "tool-exec",
          state: "approval-requested",
          approval: { id: "a1" },
          input: { x: 1 },
        },
        renderApproval: (v) =>
          createElement("div", { "data-id": v.approvalId }, v.toolName),
        renderCard,
      }),
    );
    expect(html).toContain('data-id="a1"');
    expect(html).toContain("exec");
    expect(renderCard).not.toHaveBeenCalled();
  });

  it("routes other tool states to card slot", () => {
    const html = renderToStaticMarkup(
      createElement(AiSdkToolPartSwitch, {
        part: {
          type: "tool-list_directory",
          state: "output-available",
          input: {},
        },
        renderApproval: () => null,
        renderCard: (v) =>
          createElement("div", {
            "data-open": String(v.defaultOpen),
            "data-name": v.toolName,
          }),
      }),
    );
    expect(html).toContain('data-open="true"');
    expect(html).toContain("list_directory");
  });
});
