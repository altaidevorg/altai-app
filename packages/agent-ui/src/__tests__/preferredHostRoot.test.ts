import { describe, expect, it } from "vitest";
import {
  PREFERRED_HOST_ROOT_STATE_KEY,
  readPreferredHostRootFromState,
  retainPreferredHostRootUri,
} from "../lib/preferredHostRoot.js";

describe("preferredHostRoot", () => {
  it("retains only open roots", () => {
    expect(retainPreferredHostRootUri("file:///a", ["file:///a"])).toBe(
      "file:///a",
    );
    expect(retainPreferredHostRootUri("file:///a", ["file:///b"])).toBe(
      undefined,
    );
  });
  it("reads from memento-like state", () => {
    const state = {
      get: (k: string) =>
        k === PREFERRED_HOST_ROOT_STATE_KEY ? "file:///x" : undefined,
    };
    expect(readPreferredHostRootFromState(state, ["file:///x"])).toBe(
      "file:///x",
    );
  });
});
