import { describe, expect, it } from "vitest";
import { withAssetCacheBust } from "../lib/assetCacheBust.js";

describe("withAssetCacheBust", () => {
  it("appends query", () => {
    expect(withAssetCacheBust("file:///main.js", 1)).toBe("file:///main.js?v=1");
    expect(withAssetCacheBust("file:///main.js?x=1", 2)).toBe(
      "file:///main.js?x=1&v=2",
    );
  });
});
