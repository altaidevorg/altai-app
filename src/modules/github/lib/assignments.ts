import { LazyStore } from "@tauri-apps/plugin-store";
import { z } from "zod";
import { PERMISSION_MODES, type PermissionMode } from "@/modules/settings/store";

/** What an agent run was assigned to work on. */
export type AssignmentSource =
  | { kind: "issue"; owner: string; repo: string; number: number; url: string }
  | { kind: "pr"; owner: string; repo: string; number: number; url: string }
  | { kind: "todo"; todoId: string }
  /** A user-created task that runs in its own background chat. */
  | { kind: "task"; prompt: string };

export type AssignmentStatus =
  | "dispatching"
  | "running"
  | "awaiting-approval"
  | "done"
  | "failed"
  | "cancelled";

export type AssignmentDelivery =
  | {
      status: "worktree" | "publishing" | "failed";
      workspacePath: string;
      branchName: string;
      baseBranch: string;
      error?: string;
    }
  | {
      status: "draft-pr";
      workspacePath: string;
      branchName: string;
      baseBranch: string;
      pullNumber: number;
      pullUrl: string;
    };

/** Per-task runtime choices. They are deliberately stored with the task so a
 * completed run remains auditable after the global preferences change. */
export type AssignmentRunConfig = {
  agentId?: string;
  modelId?: string;
  skills?: string[];
  permissionMode?: PermissionMode;
  /** Per-assignment sandbox override, normally an ALTAI-created git worktree. */
  workspacePath?: string;
  /** Branch checked out at workspacePath, retained for delivery/recovery UI. */
  branchName?: string;
  /** Branch the worktree was created from. */
  baseBranch?: string;
};

/** One assignment = one ALTAI session = one IsanAgent chat_id (1:1). */
export interface Assignment {
  id: string;
  source: AssignmentSource;
  /** The ALTAI session / IsanAgent chat_id driving this work. */
  sessionId: string;
  title: string;
  status: AssignmentStatus;
  runConfig?: AssignmentRunConfig;
  /** Delivery state for issue runs isolated in an ALTAI git worktree. */
  delivery?: AssignmentDelivery;
  createdAt: number;
  updatedAt: number;
}

const assignmentSourceSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("issue"),
    owner: z.string(),
    repo: z.string(),
    number: z.number(),
    url: z.string(),
  }),
  z.object({
    kind: z.literal("pr"),
    owner: z.string(),
    repo: z.string(),
    number: z.number(),
    url: z.string(),
  }),
  z.object({ kind: z.literal("todo"), todoId: z.string() }),
  z.object({ kind: z.literal("task"), prompt: z.string().min(1) }),
]);

const assignmentSchema = z.object({
  id: z.string(),
  source: assignmentSourceSchema,
  sessionId: z.string(),
  title: z.string(),
  status: z.enum([
    "dispatching",
    "running",
    "awaiting-approval",
    "done",
    "failed",
    "cancelled",
  ]),
  runConfig: z
    .object({
      agentId: z.string().optional(),
      modelId: z.string().optional(),
      skills: z.array(z.string().min(1)).max(20).optional(),
      permissionMode: z.enum(PERMISSION_MODES).optional(),
      workspacePath: z.string().min(1).optional(),
      branchName: z.string().min(1).optional(),
      baseBranch: z.string().min(1).optional(),
    })
    .optional(),
  delivery: z
    .discriminatedUnion("status", [
      z.object({
        status: z.enum(["worktree", "publishing", "failed"]),
        workspacePath: z.string().min(1),
        branchName: z.string().min(1),
        baseBranch: z.string().min(1),
        error: z.string().optional(),
      }),
      z.object({
        status: z.literal("draft-pr"),
        workspacePath: z.string().min(1),
        branchName: z.string().min(1),
        baseBranch: z.string().min(1),
        pullNumber: z.number().int().positive(),
        pullUrl: z.string().min(1),
      }),
    ])
    .optional(),
  createdAt: z.number(),
  updatedAt: z.number(),
}) satisfies z.ZodType<Assignment>;

const STORE_PATH = "altai-assignments.json";
const KEY = "assignments";
const store = new LazyStore(STORE_PATH, { defaults: {}, autoSave: 200 });

export async function loadAssignments(): Promise<Assignment[]> {
  const list = await store.get<unknown>(KEY);
  if (!Array.isArray(list)) return [];
  // Keep only entries that satisfy the schema — a corrupt or partially-written
  // store yields an empty list rather than poisoning downstream consumers.
  return list.flatMap((entry) => {
    const parsed = assignmentSchema.safeParse(entry);
    return parsed.success ? [parsed.data] : [];
  });
}

export async function saveAssignments(list: Assignment[]): Promise<void> {
  await store.set(KEY, list);
  await store.save();
}

function clip(text: string, max = 4000): string {
  const t = text.trim();
  return t.length > max ? `${t.slice(0, max)}\n…(truncated)` : t;
}

/** Seed prompt for an issue/PR assignment. */
export function buildItemSeed(input: {
  kind: "issue" | "pr";
  owner: string;
  repo: string;
  number: number;
  title: string;
  body: string | null;
  additionalInstructions?: string;
  worktree?: { path: string; branch: string; baseBranch: string };
}): string {
  const noun = input.kind === "pr" ? "pull request" : "issue";
  const verb =
    input.kind === "pr"
      ? "Review it, address any problems, and push the needed changes."
      : "Investigate and complete it end-to-end.";
  const additionalInstructions = input.additionalInstructions?.trim();
  return [
    `You've been assigned to work on a GitHub ${noun}.`,
    ``,
    `Repository: ${input.owner}/${input.repo}`,
    `${noun === "issue" ? "Issue" : "PR"} #${input.number}: ${input.title}`,
    ``,
    input.body ? clip(input.body) : "(no description provided)",
    ``,
    input.worktree
      ? [
          "Isolated delivery workspace:",
          `- Worktree: ${input.worktree.path}`,
          `- Branch: ${input.worktree.branch}`,
          `- Base branch: ${input.worktree.baseBranch}`,
          "Stay on this branch and keep all edits inside this worktree. Do not modify the user's base working tree.",
          "",
        ].join("\n")
      : "",
    additionalInstructions
      ? `Additional instructions:\n${clip(additionalInstructions)}\n`
      : "",
    `${verb} Use todo_write to lay out your plan, and spawn sub-agents for independent parts as needed. When finished, summarize what you did.`,
  ].join("\n");
}

/** Seed prompt for a local todo assignment. */
export function buildTodoSeed(title: string, description?: string): string {
  return [
    `You've been assigned to complete this task:`,
    ``,
    title,
    description ? `\n${clip(description)}` : "",
    ``,
    `Use todo_write to track sub-steps and spawn sub-agents where it helps. Summarize the outcome when done.`,
  ].join("\n");
}

/** Seed prompt for a standalone task run. */
export function buildTaskSeed(prompt: string, skills?: string[]): string {
  return [
    "You are running as an independent background task in ALTAI.",
    "",
    "Task:",
    clip(prompt, 12_000),
    "",
    skills?.length
      ? `Selected workspace skills: ${skills.join(", ")}. Load the relevant skill instructions before using a selected skill.`
      : "",
    "Work autonomously in the current workspace. Start with todo_write for any task with multiple steps. Inspect before changing files, verify your work when practical, and ask for approval only when an action genuinely requires it. Finish with a concise summary of results, changed files, and any remaining limitations.",
  ].join("\n");
}
