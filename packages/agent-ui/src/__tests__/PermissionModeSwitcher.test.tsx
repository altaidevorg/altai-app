import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  effectivePermissionMode,
  PermissionModeSwitcher,
  visiblePermissionModes,
} from "../components/PermissionModeSwitcher.js";

describe("effectivePermissionMode", () => {
  it("downgrades bypass when the gate is locked", () => {
    expect(effectivePermissionMode("bypass", false)).toBe("ask");
    expect(effectivePermissionMode("bypass", true)).toBe("bypass");
    expect(effectivePermissionMode("plan", false)).toBe("plan");
  });
});

describe("visiblePermissionModes", () => {
  it("omits bypass when the capability is gated off", () => {
    expect(visiblePermissionModes(true)).toContain("bypass");
    expect(visiblePermissionModes(false)).not.toContain("bypass");
  });
});

describe("PermissionModeSwitcher", () => {
  it("renders the effective mode label on the trigger", () => {
    const html = renderToStaticMarkup(
      createElement(PermissionModeSwitcher, {
        mode: "bypass",
        bypassEnabled: false,
        onSelectMode: () => {},
      }),
    );
    expect(html).toContain('aria-label="Permission mode: Ask before edit"');
    expect(html).toContain("Ask before edit");
    expect(html).toContain('aria-haspopup="menu"');
  });
});
