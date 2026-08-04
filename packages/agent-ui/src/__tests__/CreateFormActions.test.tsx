import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { CreateFormActions } from "../components/CreateFormActions.js";

describe("CreateFormActions", () => {
  it("renders sectioned task footer", () => {
    const html = renderToStaticMarkup(
      createElement(CreateFormActions, {
        status: "Something failed",
        statusTone: "destructive",
        onCancel: () => {},
        submitLabel: "Run in background",
        submitDisabled: true,
        sectioned: true,
      }),
    );
    expect(html).toContain("Something failed");
    expect(html).toContain("Cancel");
    expect(html).toContain("Run in background");
    expect(html).toContain("disabled");
  });

  it("renders inline automation footer status", () => {
    const html = renderToStaticMarkup(
      createElement(CreateFormActions, {
        status: "Schedule is ready",
        onCancel: () => {},
        submitLabel: "Create",
      }),
    );
    expect(html).toContain("Schedule is ready");
    expect(html).toContain("Create");
  });
});
