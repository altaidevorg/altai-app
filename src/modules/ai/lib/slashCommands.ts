import {
  Archive02Icon,
  CheckListIcon,
  File01Icon,
  SparklesIcon,
} from "@hugeicons/core-free-icons";
import { usePlanStore } from "../store/planStore";
import { useChatStore } from "../store/chatStore";

/**
 * Outcome of intercepting a slash command from the composer.
 *
 * - `"handled"`: command ran; the composer should NOT send a chat message.
 * - `"send-prompt"`: replace the user's text with `prompt` and send normally.
 * - `"none"`: not a slash command; let the composer behave as usual.
 */
export type SlashOutcome =
  | { kind: "handled"; toast?: string }
  | { kind: "send-prompt"; prompt: string; commandName?: string }
  | { kind: "none" };

const INIT_PROMPT = `Scan this workspace and produce ALTAI.md at the workspace root with:

- One-paragraph project description.
- Build / test / dev commands.
- Architecture overview (subsystems, data flow, key dirs).
- Conventions worth knowing (naming, patterns, gotchas).
- Paths to entry points.

Use grep/glob/list_directory/read_file to explore. Cap ALTAI.md under 200 lines. Use write_file to create it (will go through normal approval).`;

/** Prompt that triggers a between-turns compaction via the registered
 *  `compact_context` tool. Aliases (`smol`, `condense`) route here too. */
const COMPACT_PROMPT =
  "Run the compact_context tool now to summarize our conversation history so far, keeping the most recent turns intact. Do not ask for confirmation — compact immediately.";

export type SlashCommandMeta = {
  name: string;
  invocation: string;
  label: string;
  icon: typeof SparklesIcon;
};

export const SLASH_COMMANDS: Record<string, SlashCommandMeta> = {
  init: {
    name: "init",
    invocation: "/init",
    label: "Initialize workspace",
    icon: SparklesIcon,
  },
  plan: {
    name: "plan",
    invocation: "/plan",
    label: "Plan mode",
    icon: CheckListIcon,
  },
  paper: {
    name: "paper",
    invocation: "/paper",
    label: "Import arXiv paper",
    icon: File01Icon,
  },
  compact: {
    name: "compact",
    invocation: "/compact",
    label: "Compact context",
    icon: Archive02Icon,
  },
};

export const ALTAI_CMD_RE =
  /^<altai-command\s+name="([a-z0-9-]+)"(?:\s+state="([a-z]+)")?\s*\/>(?:\n+|$)/;

export function wrapWithCommandMarker(prompt: string, name: string): string {
  return `<altai-command name="${name}" />\n\n${prompt}`;
}

export function tryRunSlashCommand(input: string): SlashOutcome {
  const trimmed = input.trim();
  const lead = trimmed[0];
  if (lead !== "/" && lead !== "#") return { kind: "none" };
  const [head, ...rest] = trimmed.slice(1).split(/\s+/);
  if (lead === "#" && !SLASH_COMMANDS[head]) return { kind: "none" };
  const tail = rest.join(" ").trim();

  switch (head) {
    case "plan": {
      const store = usePlanStore.getState();
      if (tail === "off" || tail === "exit") {
        store.disable();
        return { kind: "handled", toast: "Plan mode off" };
      }
      store.toggle();
      const nowActive = usePlanStore.getState().active;
      return {
        kind: "handled",
        toast: nowActive ? "Plan mode on" : "Plan mode off",
      };
    }
    case "init": {
      return {
        kind: "send-prompt",
        prompt: INIT_PROMPT,
        commandName: "init",
      };
    }
    case "paper": {
      useChatStore.getState().setPaperImportOpen(true);
      return { kind: "handled" };
    }
    case "compact":
    case "smol":
    case "condense": {
      return {
        kind: "send-prompt",
        prompt: COMPACT_PROMPT,
        commandName: "compact",
      };
    }
    default:
      return { kind: "none" };
  }
}

/**
 * Fire a manual `/compact` directly (no input prefill, no Enter required).
 * Resolves the slash command into its prompt and sends it through the normal
 * chat send path so the model invokes the registered `compact_context` tool
 * and the transcript shows the tool card. Safe to call from any UI surface
 * (status-bar button, Settings tab CTA). No-op when no chat is active.
 */
export async function runCompactNow(): Promise<void> {
  const outcome = tryRunSlashCommand("/compact");
  if (outcome.kind !== "send-prompt") return;
  const marker = outcome.commandName
    ? `<altai-command name="${outcome.commandName}" />\n\n`
    : "";
  const { sendMessage } = await import("../store/chatStore");
  await sendMessage(`${marker}${outcome.prompt}`);
}
