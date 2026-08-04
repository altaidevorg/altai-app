import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { RunRecoveryActions } from "../components/RunRecoveryActions.js";

describe("RunRecoveryActions", () => {
  it("renders warning controls", () => {
    const html = renderToStaticMarkup(
      createElement(RunRecoveryActions, {
        warning: true,
        title: "Possible repeated failure",
        detail: "Still working",
        canContinue: false,
        canRetry: false,
        onContinue: () => {},
        onRetry: () => {},
        onSteer: () => {},
        onStop: () => {},
        onDismiss: () => {},
      }),
    );
    expect(html).toContain('role="status"');
    expect(html).toContain("Possible repeated failure");
    expect(html).toContain("Still working");
    expect(html).toContain("Steer");
    expect(html).toContain("Stop");
    expect(html).toContain("Dismiss");
    expect(html).not.toContain("Continue");
    expect(html).not.toContain("Retry");
  });

  it("renders continue and retry for completed outcomes", () => {
    const html = renderToStaticMarkup(
      createElement(RunRecoveryActions, {
        warning: false,
        title: "Turn limit reached",
        detail: "Hit the turn limit",
        canContinue: true,
        canRetry: true,
        onContinue: () => {},
        onRetry: () => {},
        onSteer: () => {},
        onStop: () => {},
        onDismiss: () => {},
      }),
    );
    expect(html).toContain('role="alert"');
    expect(html).toContain("Continue");
    expect(html).toContain("Retry");
    expect(html).toContain("Steer");
    expect(html).not.toContain("Stop");
    expect(html).not.toContain("Dismiss");
  });
});
