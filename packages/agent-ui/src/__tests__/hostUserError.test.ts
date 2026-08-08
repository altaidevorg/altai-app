import { describe, expect, it } from "vitest";
import {
  extractHostErrorCode,
  formatHostUserError,
  isJournalUnavailableError,
} from "../lib/hostUserError.js";

describe("hostUserError", () => {
  it("maps known codes", () => {
    expect(extractHostErrorCode("host_not_ready")).toBe("host_not_ready");
    expect(formatHostUserError("host_not_ready")).toContain("not ready");
  });
  it("extracts snake_case from longer messages", () => {
    expect(extractHostErrorCode(new Error("rpc: journal_unavailable"))).toBe(
      "journal_unavailable",
    );
    expect(isJournalUnavailableError("journal_unavailable")).toBe(true);
  });
  it("softens unknown snake_case", () => {
    expect(formatHostUserError("totally_unknown_code")).toContain(
      "Something went wrong",
    );
  });
});
