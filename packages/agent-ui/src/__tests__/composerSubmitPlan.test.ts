import { describe, expect, it } from "vitest";
import { getComposerActionAvailability } from "../lib/composerEnterAction.js";
import { planComposerSubmit } from "../lib/composerSubmitPlan.js";

const idleAvail = getComposerActionAvailability({
  status: "idle",
  hasDraft: true,
  hasNativeAttachment: false,
  runId: null,
  submitting: false,
});

describe("planComposerSubmit", () => {
  it("noops when empty or not allowed", () => {
    expect(
      planComposerSubmit({
        action: "send",
        availability: idleAvail,
        draft: { value: "", files: [], snippets: [], commands: [] },
        catalog: [],
      }).kind,
    ).toBe("noop");

    const busy = getComposerActionAvailability({
      status: "streaming",
      hasDraft: true,
      hasNativeAttachment: false,
      runId: "r1",
      submitting: false,
    });
    expect(
      planComposerSubmit({
        action: "send",
        availability: busy,
        draft: { value: "hi", files: [], snippets: [], commands: [] },
        catalog: [],
      }).kind,
    ).toBe("noop");
  });

  it("returns handled for local slash outcomes", () => {
    const plan = planComposerSubmit({
      action: "send",
      availability: idleAvail,
      draft: { value: "/stop", files: [], snippets: [], commands: [] },
      catalog: [],
      resolveSlash: () => ({ kind: "handled", toast: "Stopping" }),
    });
    expect(plan).toEqual({
      kind: "handled",
      clearDraft: true,
      toast: "Stopping",
    });
  });

  it("submits composed text with send-prompt expansion", () => {
    const plan = planComposerSubmit({
      action: "send",
      availability: idleAvail,
      draft: { value: "/init", files: [], snippets: [], commands: [] },
      catalog: [],
      resolveSlash: () => ({
        kind: "send-prompt",
        prompt: "Scan workspace",
        commandName: "init",
      }),
    });
    expect(plan.kind).toBe("submit");
    if (plan.kind !== "submit") return;
    expect(plan.composed).toContain('<altai-command name="init" />');
    expect(plan.composed).toContain("Scan workspace");
    expect(plan.action).toBe("send");
  });

  it("prefixes picked command names into resolveSlash source", () => {
    let seen = "";
    planComposerSubmit({
      action: "send",
      availability: idleAvail,
      draft: {
        value: "do thing",
        files: [],
        snippets: [],
        commands: [{ name: "plan" }],
      },
      catalog: [],
      resolveSlash: (source) => {
        seen = source;
        return { kind: "none" };
      },
    });
    expect(seen).toBe("#plan do thing");
  });
});
