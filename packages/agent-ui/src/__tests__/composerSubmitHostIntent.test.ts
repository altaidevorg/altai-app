import { describe, expect, it } from "vitest";
import type { ComposerSubmitPlan } from "../lib/composerSubmitPlan.js";
import { mapComposerSubmitPlanToHostIntent } from "../lib/composerSubmitHostIntent.js";

const submitPlan = (
  action: "send" | "steer" | "queue",
): Extract<ComposerSubmitPlan, { kind: "submit" }> => ({
  kind: "submit",
  action,
  composed: "hello",
  multimodal: { imageUrls: [], documents: [] },
  clearDraftOnAccept: true,
});

describe("mapComposerSubmitPlanToHostIntent", () => {
  it("forwards noop and handled plans", () => {
    expect(
      mapComposerSubmitPlanToHostIntent({ kind: "noop" }, {
        sessionId: "s",
        runId: "r",
      }),
    ).toEqual({ kind: "noop" });

    expect(
      mapComposerSubmitPlanToHostIntent(
        { kind: "handled", clearDraft: true, toast: "ok" },
        { sessionId: "s", runId: null },
      ),
    ).toEqual({ kind: "handled", clearDraft: true, toast: "ok" });
  });

  it("requires sessionId for send/queue", () => {
    expect(
      mapComposerSubmitPlanToHostIntent(submitPlan("send"), {
        sessionId: null,
        runId: null,
      }),
    ).toEqual({ kind: "noop" });

    const send = mapComposerSubmitPlanToHostIntent(submitPlan("send"), {
      sessionId: "s1",
      runId: null,
    });
    expect(send).toMatchObject({
      kind: "send",
      queue: false,
      sessionId: "s1",
      composed: "hello",
    });

    const queue = mapComposerSubmitPlanToHostIntent(submitPlan("queue"), {
      sessionId: "s1",
      runId: "r",
    });
    expect(queue).toMatchObject({ kind: "send", action: "queue", queue: true });
  });

  it("requires sessionId and runId for steer", () => {
    expect(
      mapComposerSubmitPlanToHostIntent(submitPlan("steer"), {
        sessionId: "s1",
        runId: null,
      }),
    ).toEqual({ kind: "noop" });

    expect(
      mapComposerSubmitPlanToHostIntent(submitPlan("steer"), {
        sessionId: "s1",
        runId: "run-1",
      }),
    ).toMatchObject({
      kind: "steer",
      sessionId: "s1",
      runId: "run-1",
      composed: "hello",
    });
  });
});
