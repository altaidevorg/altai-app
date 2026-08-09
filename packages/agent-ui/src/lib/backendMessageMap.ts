/**
 * Pure backend OpenAI-style chat message → transcript parts (A6.165).
 * Host casts role/parts into its UIMessage runtime type.
 */

export type BackendChatMessage = {
  role: string;
  content?: string | null;
  tool_calls?: Array<{
    id: string;
    function: { name: string; arguments: string };
  }> | null;
  tool_call_id?: string | null;
  reasoning_content?: string | null;
};

export type TranscriptTextPart = { type: "text"; text: string };

export type TranscriptToolPart = {
  type: "dynamic-tool";
  toolName: string;
  toolCallId: string;
  input: Record<string, unknown>;
  state: "input-available";
};

export type TranscriptPart = TranscriptTextPart | TranscriptToolPart;

export type TranscriptMessage = {
  id: string;
  role: string;
  parts: TranscriptPart[];
};

/**
 * Map a backend chat message to a host-agnostic transcript message.
 * Tool results stay as text parts; tool call id is not re-emitted as a part field.
 */
export function mapBackendMessageToTranscript(
  msg: BackendChatMessage,
  index: number,
): TranscriptMessage {
  const parts: TranscriptPart[] = [];
  const reasoning = msg.reasoning_content?.trim();
  if (reasoning) {
    parts.push({ type: "text", text: reasoning });
  }
  if (msg.tool_calls && msg.tool_calls.length > 0) {
    for (const tc of msg.tool_calls) {
      let input: Record<string, unknown> = {};
      try {
        input = JSON.parse(tc.function.arguments || "{}") as Record<
          string,
          unknown
        >;
      } catch {
        input = { raw: tc.function.arguments };
      }
      parts.push({
        type: "dynamic-tool",
        toolName: tc.function.name,
        toolCallId: tc.id,
        input,
        state: "input-available",
      });
    }
  }
  const text = typeof msg.content === "string" ? msg.content.trim() : "";
  if (text) {
    parts.push({ type: "text", text });
  }
  return {
    id: `backend-${index}`,
    role: msg.role === "tool" ? "assistant" : msg.role,
    parts,
  };
}
