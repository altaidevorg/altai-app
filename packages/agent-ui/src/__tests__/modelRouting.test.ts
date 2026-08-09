import { describe, expect, it } from "vitest";
import {
  pickAutoModel,
  supportsAgentModel,
  type SharedModelRoutingInfo,
} from "../lib/modelRouting.js";

const coder = { id: "builtin:coder", name: "Coder" };

const fastToolModel: SharedModelRoutingInfo = {
  id: "fast-tool",
  label: "Fast tool model",
  capabilities: { intelligence: 3, speed: 5, cost: 5 },
  tags: ["tools", "coding"],
};

const reasoningToolModel: SharedModelRoutingInfo = {
  id: "reasoning-tool",
  label: "Reasoning tool model",
  capabilities: { intelligence: 5, speed: 2, cost: 2 },
  tags: ["reasoning", "tools", "coding"],
};

const chatOnlyModel: SharedModelRoutingInfo = {
  id: "chat-only",
  label: "Chat only model",
  capabilities: { intelligence: 5, speed: 5, cost: 5 },
  tags: ["vision"],
};

describe("modelRouting (A6.140)", () => {
  it("keeps explicitly non-tool-capable models out of agent runs", () => {
    expect(supportsAgentModel(chatOnlyModel, coder)).toBe(false);
    expect(supportsAgentModel(fastToolModel, coder)).toBe(true);
  });

  it("uses fast compatible models for short tasks and reasoning models for complex work", () => {
    const models = [fastToolModel, reasoningToolModel, chatOnlyModel];
    expect(
      pickAutoModel({ models, agent: coder, prompt: "Rename this variable" })
        ?.id,
    ).toBe("fast-tool");
    expect(
      pickAutoModel({
        models,
        agent: coder,
        prompt:
          "Plan a security migration for this multi-step refactor",
      })?.id,
    ).toBe("reasoning-tool");
  });
});
