import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ChatProjectTarget } from "../components/ChatProjectTarget.js";

describe("ChatProjectTarget", () => {
  it("renders choose-project empty state", () => {
    const html = renderToStaticMarkup(
      createElement(ChatProjectTarget, {
        name: "",
        path: null,
        kind: null,
        onChange: () => {},
      }),
    );
    expect(html).toContain("Choose a project");
    expect(html).toContain("Optional · Local folder or GitHub");
    expect(html).toContain('aria-label="Choose a project"');
  });

  it("renders local folder selection", () => {
    const html = renderToStaticMarkup(
      createElement(ChatProjectTarget, {
        name: "altai-app",
        path: "/Users/me/altai-app",
        kind: "local",
        onChange: () => {},
      }),
    );
    expect(html).toContain("altai-app");
    expect(html).toContain("Local folder");
    expect(html).toContain("Change project, currently altai-app");
  });

  it("renders github selection", () => {
    const html = renderToStaticMarkup(
      createElement(ChatProjectTarget, {
        name: "altaidevorg/altai-app",
        path: "github:altaidevorg/altai-app",
        kind: "github",
        onChange: () => {},
      }),
    );
    expect(html).toContain("GitHub repository");
    expect(html).toContain("altaidevorg/altai-app");
  });
});
