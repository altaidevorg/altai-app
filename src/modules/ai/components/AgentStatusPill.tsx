import { toolLabel } from "@/components/ai-elements/tool";
import {
  AgentStatusPill as AgentStatusPillView,
  type AgentStatusPillProps,
} from "@altai/agent-ui";
import { isRecoverableAttentionMessage } from "../lib/agentEventBridge";
import { useChatStore } from "../store/chatStore";

type Props = Omit<
  AgentStatusPillProps,
  "meta" | "formatStepLabel" | "isRecoverableAttention"
>;

/**
 * Desktop adapter: binds the shared status pill to chatStore + tool labels.
 */
export function AgentStatusPill(props: Props) {
  const meta = useChatStore((s) => s.agentMeta);
  return (
    <AgentStatusPillView
      {...props}
      meta={{
        status: meta.status,
        step: meta.step,
        approvalsPending: meta.approvalsPending,
        error: meta.error,
        activeSubagentCount: meta.activeSubagents.length,
      }}
      formatStepLabel={toolLabel}
      isRecoverableAttention={isRecoverableAttentionMessage}
    />
  );
}
