import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ComposerPrimaryRow } from "../components/ComposerPrimaryRow.js";

describe("ComposerPrimaryRow", () => {
  it("renders host-provided tools, permission, and submit controls", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerPrimaryRow, {
        tools: createElement("button", null, "Attach"),
        permission: createElement("button", null, "Ask"),
        submit: createElement("button", null, "Send"),
      }),
    );

    expect(html).toContain("altai-ai-composer-primary");
    expect(html).toContain("altai-ai-composer-tools");
    expect(html).toContain("altai-ai-composer-permission-bottom");
    expect(html).toContain("altai-ai-composer-submit");
    expect(html).toContain("Attach");
    expect(html).toContain("Ask");
    expect(html).toContain("Send");
  });

  it("omits the optional permission slot", () => {
    const html = renderToStaticMarkup(
      createElement(ComposerPrimaryRow, {
        tools: "Tools",
        submit: "Stop",
      }),
    );

    expect(html).not.toContain("altai-ai-composer-permission-bottom");
    expect(html).toContain("Stop");
  });
});
