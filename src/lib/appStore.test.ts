import { describe, expect, it, vi } from "vitest";
import { createAppStore } from "./appStore";

describe("AppStore browser fallback", () => {
  it("persists values and exposes entries without invoking Tauri", async () => {
    const path = `test-${crypto.randomUUID()}.json`;
    const store = createAppStore(path, { defaults: { theme: "system" } });

    await store.set("theme", "dark");
    await store.set("zoom", 1.25);

    expect(await store.get("theme")).toBe("dark");
    expect(await store.entries()).toEqual([
      ["theme", "dark"],
      ["zoom", 1.25],
    ]);
  });

  it("notifies browser listeners for set and delete", async () => {
    const path = `test-${crypto.randomUUID()}.json`;
    const store = createAppStore(path);
    const onChange = vi.fn();
    const unlisten = await store.onChange(onChange);

    await store.set("agent", "coder");
    await store.delete("agent");

    expect(onChange).toHaveBeenNthCalledWith(1, "agent", "coder");
    expect(onChange).toHaveBeenNthCalledWith(2, "agent", undefined);
    unlisten();
  });
});
