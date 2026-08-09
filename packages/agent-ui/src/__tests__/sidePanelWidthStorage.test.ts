import { describe, expect, it } from "vitest";
import {
  readPanelWidthFromStorage,
  writePanelWidthToStorage,
} from "../lib/sidePanelWidthStorage.js";

describe("sidePanelWidthStorage", () => {
  it("reads/writes through inject storage", () => {
    const map = new Map<string, string>();
    const storage = {
      getItem: (k: string) => map.get(k) ?? null,
      setItem: (k: string, v: string) => {
        map.set(k, v);
      },
    };
    writePanelWidthToStorage(storage, "w", 240, 176, 360);
    expect(readPanelWidthFromStorage(storage, "w", 200, 176, 360)).toBe(240);
  });
});
