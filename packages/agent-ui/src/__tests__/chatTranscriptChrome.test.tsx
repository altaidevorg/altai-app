import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { AiChatTranscriptFrame } from "../components/AiChatTranscriptFrame.js";
import {
  canRetryLastAssistantTurn,
  isRecoverableAttentionMessage,
  isRetryableRunOutcome,
  joinMessageTextParts,
  resolveChatAriaLive,
  resolveTranscriptRunErrorVariant,
} from "../lib/chatTranscriptChrome.js";

describe("chatTranscriptChrome", () => {
  it("maps announce prefs", () => {
    expect(resolveChatAriaLive("off")).toBe("off");
    expect(resolveChatAriaLive("assertive")).toBe("assertive");
    expect(resolveChatAriaLive("polite")).toBe("polite");
  });

  it("gates last-assistant retry", () => {
    expect(
      canRetryLastAssistantTurn({
        retryableFailure: true,
        role: "assistant",
        index: 1,
        messageCount: 2,
        status: "ready",
      }),
    ).toBe(true);
    expect(
      canRetryLastAssistantTurn({
        retryableFailure: true,
        role: "assistant",
        index: 1,
        messageCount: 2,
        status: "streaming",
      }),
    ).toBe(false);
  });

  it("joins text parts and classifies errors", () => {
    expect(
      joinMessageTextParts([
        { type: "text", text: "a" },
        { type: "tool" },
        { type: "text", text: "b" },
      ]),
    ).toBe("a\nb");
    expect(isRecoverableAttentionMessage("Run paused — x")).toBe(true);
    expect(resolveTranscriptRunErrorVariant("Run paused — x")).toBe(
      "attention",
    );
    expect(isRetryableRunOutcome({ kind: "failed", retryable: true })).toBe(
      true,
    );
  });
});

describe("AiChatTranscriptFrame", () => {
  it("renders empty or filled layouts", () => {
    const empty = renderToStaticMarkup(
      createElement(AiChatTranscriptFrame, {
        isEmpty: true,
        empty: createElement("div", null, "empty-home"),
      }),
    );
    expect(empty).toContain("empty-home");
    expect(empty).toContain("altai-ai-transcript-empty");

    const filled = renderToStaticMarkup(
      createElement(
        AiChatTranscriptFrame,
        {
          isEmpty: false,
          end: createElement("footer", null, "status"),
        },
        createElement("p", null, "msg"),
      ),
    );
    expect(filled).toContain("msg");
    expect(filled).toContain("status");
    expect(filled).toContain("altai-ai-transcript");
  });
});
