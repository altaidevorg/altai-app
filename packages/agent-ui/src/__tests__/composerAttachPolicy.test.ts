import { describe, expect, it } from "vitest";
import {
  canMountComposerAttachMenu,
  composerAttachSurfaceShowsAttachments,
  composerAttachSurfaceShowsToolbar,
} from "../lib/composerAttachPolicy.js";

describe("composerAttachPolicy", () => {
  it("mounts when any attach capability is true", () => {
    expect(
      canMountComposerAttachMenu({
        canActiveFile: false,
        canSelection: true,
        canGitDiff: false,
        canTerminal: false,
      }),
    ).toBe(true);
    expect(
      canMountComposerAttachMenu({
        canActiveFile: false,
        canSelection: false,
        canGitDiff: false,
        canTerminal: false,
      }),
    ).toBe(false);
  });

  it("resolves surface flags", () => {
    expect(composerAttachSurfaceShowsAttachments("toolbar")).toBe(false);
    expect(composerAttachSurfaceShowsToolbar("toolbar")).toBe(true);
    expect(composerAttachSurfaceShowsAttachments("all")).toBe(true);
  });
});
