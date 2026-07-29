import { describe, expect, it } from "vitest";
import type { Agent } from "./agents";
import type { ModelInfo } from "../config";
import { pickAutoModel, supportsAgentModel } from "./modelRouting";

const coder: Agent = {
  id: "builtin:coder",
  name: "Coder",
  description: "",
  instructions: "",
  icon: "coder",
  builtIn: true,
};

const fastToolModel: ModelInfo = {
  id: "fast-tool",
  provider: "openai",
  label: "Fast tool model",
  hint: "Fast",
  description: "",
  capabilities: { intelligence: 3, speed: 5, cost: 5 },
  tags: ["tools", "coding"],
};

const reasoningToolModel: ModelInfo = {
  id: "reasoning-tool",
  provider: "openai",
  label: "Reasoning tool model",
  hint: "Reasoning",
  description: "",
  capabilities: { intelligence: 5, speed: 2, cost: 2 },
  tags: ["reasoning", "tools", "coding"],
};

const chatOnlyModel: ModelInfo = {
  id: "chat-only",
  provider: "openai",
  label: "Chat only model",
  hint: "Chat",
  description: "",
  capabilities: { intelligence: 5, speed: 5, cost: 5 },
  tags: ["vision"],
};

describe("model routing", () => {
  it("keeps explicitly non-tool-capable models out of agent runs", () => {
    expect(supportsAgentModel(chatOnlyModel, coder)).toBe(false);
    expect(supportsAgentModel(fastToolModel, coder)).toBe(true);
  });

  it("uses fast compatible models for short tasks and reasoning models for complex work", () => {
    const models = [fastToolModel, reasoningToolModel, chatOnlyModel];
    expect(pickAutoModel({ models, agent: coder, prompt: "Rename this variable" })?.id).toBe("fast-tool");
    expect(pickAutoModel({ models, agent: coder, prompt: "Plan a security migration for this multi-step refactor" })?.id).toBe("reasoning-tool");
  });
});
