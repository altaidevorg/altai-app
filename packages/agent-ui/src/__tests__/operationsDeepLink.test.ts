import { describe, expect, it } from "vitest";
import {
  buildOpenOperationsPayload,
  parseOpenOperationsPayload,
} from "../lib/operationsDeepLink.js";

describe("operationsDeepLink", () => {
  it("parses valid payload", () => {
    expect(
      parseOpenOperationsPayload({
        key: 1,
        view: "work",
        workHubView: "runs",
        composeTask: true,
      }),
    ).toEqual({
      key: 1,
      view: "work",
      workHubView: "runs",
      composeTask: true,
    });
  });
  it("defaults view on build", () => {
    expect(buildOpenOperationsPayload({ key: 2 }).view).toBe("overview");
  });
});
