import { describe, expect, it } from "vitest";
import {
  canMountMcpStatus,
  mcpServerStatusCopy,
  mcpSummaryCopy,
  sortMcpServersForDisplay,
} from "../lib/mcpStatusChrome.js";

describe("canMountMcpStatus", () => {
  it("requires mcp.list", () => {
    expect(canMountMcpStatus({ mcpList: true })).toBe(true);
    expect(canMountMcpStatus({ mcpList: false })).toBe(false);
  });
});

describe("mcp status helpers", () => {
  it("sorts errors and disconnected first", () => {
    const sorted = sortMcpServersForDisplay([
      {
        id: "a",
        name: "A",
        enabled: true,
        connected: true,
      },
      {
        id: "b",
        name: "B",
        enabled: true,
        connected: false,
      },
      {
        id: "c",
        name: "C",
        enabled: true,
        connected: false,
        error: "boom",
      },
    ]);
    expect(sorted.map((s) => s.id)).toEqual(["c", "b", "a"]);
  });

  it("formats copy", () => {
    expect(
      mcpServerStatusCopy({
        id: "x",
        name: "X",
        enabled: true,
        connected: true,
      }),
    ).toBe("Connected");
    expect(
      mcpSummaryCopy([
        { id: "1", name: "a", enabled: true, connected: true },
        { id: "2", name: "b", enabled: true, connected: false },
      ]),
    ).toBe("1/2 MCP connected");
  });
});
