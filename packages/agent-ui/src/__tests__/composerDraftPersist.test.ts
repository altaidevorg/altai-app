import { describe, expect, it, vi } from "vitest";
import { createComposerDraftPersistence } from "../lib/composerDraftPersist.js";

function fakeTimers() {
  let nextId = 1;
  const pending = new Map<number, { fn: () => void }>();
  return {
    timers: {
      setTimeout: (fn: () => void) => {
        const id = nextId++;
        pending.set(id, { fn });
        return id;
      },
      clearTimeout: (id: number) => {
        pending.delete(id);
      },
    },
    flushAll() {
      for (const [id, entry] of [...pending.entries()]) {
        pending.delete(id);
        entry.fn();
      }
    },
    count() {
      return pending.size;
    },
  };
}

describe("createComposerDraftPersistence", () => {
  it("flushes immediately when the host policy requests it", () => {
    const persist = vi.fn();
    const clock = fakeTimers();
    const draft = createComposerDraftPersistence(persist, clock.timers, {
      debounceMs: 200,
      shouldPersistImmediately: (value) => value.length === 0,
    });

    draft.onChange("");

    expect(persist).toHaveBeenCalledWith("");
    expect(clock.count()).toBe(0);
  });

  it("keeps only the latest draft until the debounce timer flushes", () => {
    const persist = vi.fn();
    const clock = fakeTimers();
    const draft = createComposerDraftPersistence(persist, clock.timers, {
      debounceMs: 200,
      shouldPersistImmediately: () => false,
    });

    draft.onChange("a");
    draft.onChange("ab");
    clock.flushAll();

    expect(persist).toHaveBeenCalledTimes(1);
    expect(persist).toHaveBeenCalledWith("ab");
  });
});
