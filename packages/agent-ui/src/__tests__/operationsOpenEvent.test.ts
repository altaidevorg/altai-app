import { describe, expect, it } from "vitest";
import { buildOperationsOpenIntent } from "../lib/operationsOpenEvent.js";

describe("buildOperationsOpenIntent", () => {
  it("defaults work hub to runs", () => {
    expect(buildOperationsOpenIntent("work")).toEqual({
      view: "work",
      workHubView: "runs",
    });
  });
  it("passes inbox without hub", () => {
    expect(buildOperationsOpenIntent("inbox")).toEqual({ view: "inbox" });
  });
});
