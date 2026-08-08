import { describe, expect, it } from "vitest";
import { canMountPermissionModeSwitcher } from "../lib/permissionModeChrome.js";

describe("canMountPermissionModeSwitcher", () => {
  it("requires modes + get + update", () => {
    expect(
      canMountPermissionModeSwitcher({
        permissionModes: true,
        settingsGet: true,
        settingsUpdate: true,
      }),
    ).toBe(true);
    expect(
      canMountPermissionModeSwitcher({
        permissionModes: true,
        settingsGet: true,
        settingsUpdate: false,
      }),
    ).toBe(false);
  });
});
