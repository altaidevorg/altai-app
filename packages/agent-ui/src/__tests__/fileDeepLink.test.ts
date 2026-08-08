import { describe, expect, it } from "vitest";
import {
  buildOpenChatWithFilePayload,
  parseOpenChatWithFilePayload,
} from "../lib/fileDeepLink.js";

describe("fileDeepLink", () => {
  it("parses valid payload", () => {
    expect(
      parseOpenChatWithFilePayload({
        key: 1,
        uri: "file:///a/b.ts",
        path: "/a/b.ts",
      }),
    ).toMatchObject({ name: "b.ts", path: "/a/b.ts" });
  });
  it("builds with basename default", () => {
    const p = buildOpenChatWithFilePayload({
      uri: "file:///x/y",
      path: "/x/y",
      key: 9,
    });
    expect(p).toEqual({
      key: 9,
      uri: "file:///x/y",
      path: "/x/y",
      name: "y",
    });
  });
});
