import { describe, expect, it } from "vitest";
import {
  buildOpenSettingsPayload,
  parseOpenSettingsPayload,
} from "../lib/settingsDeepLink.js";

describe("settingsDeepLink", () => {
  it("parses section", () => {
    expect(parseOpenSettingsPayload({ key: 1, section: "models" })).toEqual({
      key: 1,
      section: "models",
    });
    expect(parseOpenSettingsPayload({ key: 1, section: "Bad" })).toEqual({
      key: 1,
    });
  });
  it("builds payload", () => {
    expect(buildOpenSettingsPayload({ key: 2, section: "host" })).toEqual({
      key: 2,
      section: "host",
    });
  });
});
