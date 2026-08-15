import { describe, expect, it } from "vitest";
import type { WorkUsage } from "@altai/host-contract";
import {
  projectUsageLedger,
  summarizeUsageLedger,
} from "./usageLedgerProjection";

function usage(overrides: Partial<WorkUsage> = {}): WorkUsage {
  return {
    attemptId: "a1",
    workId: "w1",
    workTitle: "Ship the ledger",
    number: 2,
    phase: "succeeded",
    chatId: "chat_1",
    runId: "run_1",
    updatedAtMs: 9_000,
    usage: {
      promptTokens: 130,
      completionTokens: 70,
      totalTokens: 200,
      cacheReadTokens: 11,
      cacheCreationTokens: 7,
      eventCount: 2,
    },
    ...overrides,
  };
}

describe("projectUsageLedger", () => {
  it("renders attributed totals with separators on each row", () => {
    const rows = projectUsageLedger([usage()]);
    expect(rows[0].tokenLabel).toBe("200 total");
    expect(rows[0].cacheLabel).toBe("11 cache read · 7 cache write");
    expect(rows[0].attemptLabel).toBe("Attempt 2");
    expect(rows[0].phaseLabel).toBe("succeeded");
    expect(rows[0].chatId).toBe("chat_1");
  });

  it("formats large totals with thousands separators", () => {
    const rows = projectUsageLedger([
      usage({
        usage: {
          promptTokens: 1_000_000,
          completionTokens: 234_567,
          totalTokens: 1_234_567,
          cacheReadTokens: 500_000,
          cacheCreationTokens: 0,
          eventCount: 40,
        },
      }),
    ]);
    expect(rows[0].tokenLabel).toBe("1,234,567 total");
    expect(rows[0].cacheLabel).toBe("500,000 cache read · 0 cache write");
  });

  it("keeps unbound attempts distinct from zero-usage chats", () => {
    // The command guarantees usage is present whenever a chat is bound —
    // zeros when the chat recorded no usage events.
    const zeroTotals = {
      promptTokens: 0,
      completionTokens: 0,
      totalTokens: 0,
      cacheReadTokens: 0,
      cacheCreationTokens: 0,
      eventCount: 0,
    };
    const rows = projectUsageLedger([
      usage({ attemptId: "a-bound-zero", usage: zeroTotals }),
      usage({ attemptId: "a-unbound", chatId: null, runId: null, usage: null }),
    ]);
    expect(rows[0].tokenLabel).toBe("0 total");
    expect(rows[0].tokens).toEqual(zeroTotals);
    expect(rows[0].chatId).toBe("chat_1");
    expect(rows[1].tokenLabel).toBe("no chat bound");
    expect(rows[1].tokens).toBeNull();
    expect(rows[1].chatId).toBeNull();
  });

  it("drops the cache line when no cache tokens were recorded", () => {
    const rows = projectUsageLedger([
      usage({
        usage: {
          promptTokens: 10,
          completionTokens: 5,
          totalTokens: 15,
          cacheReadTokens: 0,
          cacheCreationTokens: 0,
          eventCount: 1,
        },
      }),
    ]);
    expect(rows[0].cacheLabel).toBeNull();
    expect(rows[0].tokenLabel).toBe("15 total");
  });

  it("carries the work id so a row can open the Work detail", () => {
    const rows = projectUsageLedger([usage({ workId: "w-9" })]);
    expect(rows[0].workId).toBe("w-9");
    expect(rows[0].workTitle).toBe("Ship the ledger");
  });
});

describe("summarizeUsageLedger", () => {
  it("counts attributed and unbound attempts and sums tokens", () => {
    const rows = projectUsageLedger([
      usage({ attemptId: "a1" }),
      usage({
        attemptId: "a2",
        usage: {
          promptTokens: 100,
          completionTokens: 50,
          totalTokens: 150,
          cacheReadTokens: 0,
          cacheCreationTokens: 0,
          eventCount: 1,
        },
      }),
      usage({ attemptId: "a3", chatId: null, usage: null }),
    ]);
    const summary = summarizeUsageLedger(rows);
    expect(summary).toEqual({
      attemptCount: 3,
      attributedCount: 2,
      unattributedCount: 1,
      promptTokens: 230,
      completionTokens: 120,
      totalTokens: 350,
    });
  });

  it("returns zeroed counts for an empty page", () => {
    expect(summarizeUsageLedger([])).toEqual({
      attemptCount: 0,
      attributedCount: 0,
      unattributedCount: 0,
      promptTokens: 0,
      completionTokens: 0,
      totalTokens: 0,
    });
  });
});
