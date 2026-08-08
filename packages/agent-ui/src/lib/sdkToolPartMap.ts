/**
 * Pure AI-SDK tool part → approval / name mapping (A6.43).
 * Hosts own Tool chrome; package owns shared part shape parse.
 */

export type SdkToolPartLike = {
  type?: string;
  toolName?: string;
  state?: string;
  input?: unknown;
  approval?: { id?: string };
  output?: unknown;
  errorText?: string;
};

/** Tool name from AI SDK static `tool-*` or dynamic-tool envelope. */
export function sdkToolName(part: SdkToolPartLike): string {
  if (part.type === "dynamic-tool") {
    return part.toolName ?? "";
  }
  if (typeof part.type === "string" && part.type.startsWith("tool-")) {
    return part.type.slice("tool-".length);
  }
  return part.toolName ?? "";
}

export function isSdkToolPart(part: { type?: string }): boolean {
  const t = part.type ?? "";
  return t === "dynamic-tool" || t.startsWith("tool-");
}

export type SdkToolApprovalView = {
  toolName: string;
  approvalId: string;
  input: unknown;
};

/** Map an approval-requested AI-SDK tool part for AiToolApproval. */
export function mapSdkToolApprovalPart(
  part: SdkToolPartLike,
): SdkToolApprovalView | null {
  if (part.state !== "approval-requested") return null;
  const approvalId = part.approval?.id;
  if (!approvalId) return null;
  return {
    toolName: sdkToolName(part),
    approvalId,
    input: part.input,
  };
}

export type SdkToolCardView = {
  toolName: string;
  state: string | undefined;
  input: unknown;
  output: unknown;
  errorText: string | undefined;
  defaultOpen: boolean;
};

/** Map a non-approval tool part for host Tool card props. */
export function mapSdkToolCardPart(part: SdkToolPartLike): SdkToolCardView {
  const toolName = sdkToolName(part);
  return {
    toolName,
    state: part.state,
    input: part.input,
    output: "output" in part ? part.output : undefined,
    errorText:
      typeof part.errorText === "string" ? part.errorText : undefined,
    defaultOpen: toolName === "list_directory",
  };
}
