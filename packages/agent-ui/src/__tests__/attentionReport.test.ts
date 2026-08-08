import { describe, expect, it } from "vitest";
import {
  attentionStatusBarCommand,
  parseAttentionReportParams,
} from "../lib/attentionReport.js";

describe("parseAttentionReportParams", () => {
  it("parses finite non-negative counts", () => {
    expect(parseAttentionReportParams({ count: 3.2 })).toBe(3);
    expect(parseAttentionReportParams({ count: -1 })).toBeNull();
    expect(parseAttentionReportParams({})).toBeNull();
  });
});

describe("attentionStatusBarCommand", () => {
  it("opens inbox when attention is non-zero", () => {
    expect(attentionStatusBarCommand(0)).toBe("altai.openOperations");
    expect(attentionStatusBarCommand(2)).toBe("altai.openOperationsInbox");
  });
});
