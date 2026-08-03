import { describe, expect, it, vi } from "vitest";
import { emitAppEvent, listenAppEvent } from "./appEvent";

describe("appEvent browser fallback", () => {
  it("delivers payloads and supports unsubscribe", async () => {
    const name = `test://${crypto.randomUUID()}`;
    const listener = vi.fn();
    const unlisten = await listenAppEvent<{ value: number }>(name, listener);

    await emitAppEvent(name, { value: 42 });
    expect(listener).toHaveBeenCalledWith({
      event: name,
      id: -1,
      payload: { value: 42 },
    });

    unlisten();
    await emitAppEvent(name, { value: 7 });
    expect(listener).toHaveBeenCalledTimes(1);
  });
});
