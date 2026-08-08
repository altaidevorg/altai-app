import { describe, expect, it } from "vitest";
import {
  ALTAI_COMMAND_MARKER_RE,
  indexOfLastTextPart,
  parseCommandMarkerPrefix,
  prepareUserTurnDisplay,
  resolveStreamingAssistantMessageId,
  wrapWithCommandMarker,
} from "../lib/userTurnDisplay.js";
import { composeComposerSubmitText } from "../lib/composerSubmitCompose.js";

describe("wrapWithCommandMarker / parseCommandMarkerPrefix", () => {
  it("round-trips the command name", () => {
    const marked = wrapWithCommandMarker("do the thing", "init");
    expect(ALTAI_COMMAND_MARKER_RE.test(marked)).toBe(true);
    const parsed = parseCommandMarkerPrefix(marked);
    expect(parsed.commandName).toBe("init");
    expect(parsed.rest.trim()).toBe("do the thing");
  });
});

describe("prepareUserTurnDisplay", () => {
  it("parses command marker + context chips + body", () => {
    const raw = composeComposerSubmitText({
      commandMarker: '<altai-command name="init" />',
      effectiveText: "please review",
      catalog: [],
      files: [
        {
          kind: "terminal",
          name: "zsh",
          mediaType: "text/plain",
          text: "ls\n",
        },
      ],
    });
    const display = prepareUserTurnDisplay(raw);
    expect(display.commandName).toBe("init");
    expect(display.chips.some((c) => c.kind === "terminal")).toBe(true);
    expect(display.text).toBe("please review");
  });

  it("passes plain text through", () => {
    expect(prepareUserTurnDisplay("  hello  ")).toEqual({
      commandName: null,
      commandState: null,
      text: "hello",
      chips: [],
    });
  });
});

describe("streaming helpers", () => {
  it("finds last text part index", () => {
    expect(indexOfLastTextPart([{ type: "text" }, { type: "tool" }, { type: "text" }])).toBe(
      2,
    );
    expect(indexOfLastTextPart([{ type: "tool" }])).toBe(-1);
  });

  it("resolves streaming assistant id", () => {
    expect(
      resolveStreamingAssistantMessageId(
        [
          { id: "u1", role: "user" },
          { id: "a1", role: "assistant" },
        ],
        "streaming",
      ),
    ).toBe("a1");
    expect(
      resolveStreamingAssistantMessageId(
        [{ id: "a1", role: "assistant" }],
        "ready",
      ),
    ).toBeNull();
  });
});
