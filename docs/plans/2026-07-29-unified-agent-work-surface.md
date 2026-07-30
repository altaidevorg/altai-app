# Unified Agent Work Surface

Date: 2026-07-29

Status: Proposed

Scope: AI chat navigation, background work, scheduled work, inbox, and run
inspection

## 1. Decision

ALTAI will stop presenting Task Inspector, Background Tasks, Inbox, and
Automations as four peer-level destinations.

The AI chat will expose two durable user destinations:

1. **Work**
   - **Runs**: manual and automation-created agent runs.
   - **Scheduled**: one-time and recurring automation definitions.
2. **Inbox**
   - Only items that need attention or communicate a meaningful update.

Run inspection remains available as contextual detail:

- permanently visible in the right rail on wide layouts;
- opened as **Run details** for the selected/current run on narrow layouts;
- never treated as a workspace-level task collection.

`background` is an execution property, not a navigation category.
`automation` is a persistent instruction plus trigger, not a run.

## 2. User mental model

```text
Manual delegation ---------+
                            +--> Work item / run --> Run details
Scheduled automation ------+           |
                                        +--> Event --> Inbox
```

The four concepts have separate responsibilities:

| Concept | User question | Lifetime |
| --- | --- | --- |
| Work / Runs | What are agents doing, and what did they finish? | Durable |
| Scheduled | What should run later or repeatedly? | Durable definition |
| Inbox | What needs me now, or what important result arrived? | Clearable projection |
| Run details | What happened inside this selected run? | Contextual projection |

The same underlying run may be referenced from Work and Inbox, but it must not
be duplicated as two independently managed objects. Resolving an Inbox item
must not remove its run. Removing or archiving a run must not silently mutate
unrelated notification history.

## 3. Current-state findings

### 3.1 Navigation

`AiSidePanel.tsx` models `inspector`, `tasks`, `inbox`, and `automations` as
separate overlay surfaces and exposes four adjacent controls in the chat
topbar.

This makes implementation concepts look like four independent user jobs.

### 3.2 Overlapping projections

`TaskRunsPanel.tsx` already presents:

- live work;
- attention-needed work;
- ready-to-review work;
- finished and cancelled history.

`NotificationInboxPanel.tsx` also presents:

- paused tasks;
- waiting work;
- in-progress work;
- unread and earlier updates.

`AutomationsPanel.tsx` joins automation definitions to background jobs to show
the most recent failure. The same runtime job can therefore influence
Automations and Inbox while a related assignment/run appears in Tasks.

### 3.3 Split data ownership

- Manual delegated tasks are persisted by `useAssignmentsStore`.
- Per-session live run telemetry is held by `useAgentRunsStore`.
- Notifications, clarification tickets, and background jobs are loaded by
  `useNotificationStore`.
- Automation definitions and another copy of background jobs are loaded by
  `useAutomationStore`.

The first implementation must not merely place the existing panels inside one
container. Shared status and count projections need explicit ownership, or the
new surface will preserve the current inconsistencies behind fewer buttons.

## 4. Target information architecture

### 4.1 Chat topbar

Keep:

- chat tabs;
- compact todo summary;
- **Work**;
- **Inbox** with an attention badge;
- close panel.

Contextual control:

- show **Run details** only where the right rail is not already visible;
- disable it when there is no selected session/run worth inspecting;
- do not place it inside the peer-level Work/Inbox group.

Remove as peer-level controls:

- Background Tasks;
- Automations;
- Task Inspector.

### 4.2 Work surface

Create one `WorkHubPanel` with primary tabs:

#### Runs

Default filter: `Active`

Available views:

- **Active**: dispatching, running, cancelling, or queued;
- **Needs attention**: approval, clarification, blocked, or failed;
- **Ready to review**: completed work whose result has not been accepted,
  applied, dismissed, or archived;
- **History**: completed, applied, dismissed, cancelled, and archived;
- **All**: search-oriented aggregate.

Each row/card should show:

- outcome-oriented title;
- status;
- source: manual, scheduled, GitHub, or orchestration;
- owning conversation;
- agent/model when meaningful;
- elapsed or completed time;
- current step or concise result;
- verification and changed-file summary;
- relevant primary action.

Primary actions:

- open run/transcript;
- inspect details;
- review result;
- stop active run;
- retry failed run;
- reuse instruction;
- archive/remove only when safe.

The create action is **Delegate work**, not “Run in background.” Background
execution can be described in supporting copy and configuration.

#### Scheduled

Default filter: `Active`

Available views:

- **Active**;
- **One-time**;
- **Recurring**;
- **Paused**;
- **Issues**.

Each definition should show:

- instruction/name;
- trigger or cadence;
- next run;
- last run;
- owner conversation/workspace;
- enabled/paused state;
- latest outcome;
- link to runs produced by this automation.

Primary actions:

- run now;
- pause/resume;
- edit;
- duplicate;
- delete;
- open recent runs.

One-time and recurring definitions live together because both answer “what is
scheduled?” When a definition fires, its execution also appears as a run.

### 4.3 Inbox

Inbox is a projection of events, not an operations dashboard.

Allowed Inbox item classes:

- **Action required**
  - approval request;
  - clarification question;
  - permission request;
  - blocked run;
  - failed run requiring a decision.
- **Ready to review**
  - completed work with a review/apply decision.
- **Updates**
  - meaningful completion or automation report;
  - explicitly subscribed informational events.

Remove:

- generic active/in-progress jobs;
- a generic `Work` filter;
- status browsing that is already available in Work.

Recommended filters:

- **All**;
- **Action required**;
- **Ready to review**;
- **Updates**.

Badge semantics:

- badge counts unresolved action and review items;
- ordinary unread updates do not imply that agents are blocked;
- “mark read” and “resolve” remain separate operations;
- resolving an item never deletes the referenced run.

Every Inbox card should deep-link to the canonical Work run, owning
conversation, or scheduled definition.

### 4.4 Run details

Rename the user-facing `Task inspector` to **Run details**.

Retain:

- plan/checklist;
- chronological activity;
- changes and files;
- research and connected tools;
- delegated subagent work;
- recovery/checkpoints;
- token usage and stop control.

Change:

- bind the surface to an explicit `sessionId`/`runId` where possible instead
  of relying exclusively on the globally active chat;
- opening details from Work selects the referenced run;
- closing details returns to the previous Work list state on narrow layouts.

## 5. Frontend domain/read model

Introduce pure, tested view-model types before moving UI.

Suggested location:

`src/modules/ai/lib/workView.ts`

```ts
type WorkSource =
  | { kind: "manual" }
  | { kind: "automation"; automationId: string }
  | { kind: "github"; assignmentId: string }
  | { kind: "orchestration"; taskId: string };

type WorkStatus =
  | "queued"
  | "running"
  | "waiting"
  | "blocked"
  | "failed"
  | "ready"
  | "completed"
  | "cancelled"
  | "archived";

type WorkRunView = {
  id: string;
  sessionId: string;
  runId: string | null;
  title: string;
  source: WorkSource;
  status: WorkStatus;
  requiresAction: boolean;
  readyForReview: boolean;
  createdAt: number;
  updatedAt: number;
  result: string | null;
  automationId: string | null;
};

type InboxItemView = {
  id: string;
  category: "action" | "review" | "update";
  runId: string | null;
  sessionId: string | null;
  automationId: string | null;
  read: boolean;
  resolved: boolean;
  createdAt: number;
};
```

Rules:

- adapters normalize existing assignment, run, job, ticket, notification, and
  automation records;
- React components consume the normalized model rather than re-implementing
  status rules;
- all badge and tab counts come from the same pure selectors;
- no persistence format changes are required in the first PR;
- unknown legacy job kinds remain reachable in a safe fallback state.

## 6. Implementation sequence

### PR 1 — Read model and contract tests

Goal: establish one semantic vocabulary without changing visible UI.

Add:

- `workView.ts`;
- status normalization for assignments and per-session runs;
- automation-to-job linking using the persisted automation/job identity;
- Inbox classification and deduplication selectors;
- selectors for Work tabs and Inbox badge counts.

Tests:

- manual running task maps to one active Work run;
- failed task maps to `Needs attention`;
- completed task maps to `Ready to review` until resolved/applied;
- cron job maps to its automation definition;
- clarification notification and ticket deduplicate into one Inbox item;
- an active non-waiting job does not enter Inbox;
- clearing an Inbox item does not remove its Work run.

No component should import the new model until these contract tests pass.

### PR 2 — Work hub shell and deep-link routing

Goal: add the combined destination while reusing current functionality.

Add:

- `WorkHubPanel.tsx`;
- `RunsView.tsx`, initially extracted from `TaskRunsPanel.tsx`;
- `ScheduledView.tsx`, initially extracted from `AutomationsPanel.tsx`;
- Work tab state and search persistence while the overlay stays mounted.

Change the surface event contract from a flat string to a typed destination:

```ts
type AiSurfaceTarget =
  | { surface: "history" }
  | { surface: "work"; view: "runs" | "scheduled"; itemId?: string }
  | { surface: "inbox"; itemId?: string }
  | { surface: "run-details"; sessionId?: string; runId?: string }
  | { surface: "review"; sessionId?: string };
```

Backward-compatible commands:

- `/tasks` opens `{ surface: "work", view: "runs" }`;
- `/automations` and `/schedule` open
  `{ surface: "work", view: "scheduled" }`;
- `/status`, `/activity`, and `/inspect` open `run-details`;
- `/inbox` remains Inbox;
- legacy string events are accepted for one release and normalized internally.

Do not remove old panel files until functional parity is reached.

### PR 3 — Tighten Inbox semantics

Goal: make Inbox answer only “what needs me?”

Change:

- remove active non-waiting jobs and the `Work` filter;
- add `Ready to review`;
- separate action count from ordinary unread count;
- deep-link cards into Work/Run details;
- keep ticket reply/resume and safe dismissal behavior;
- preserve ticket/notification deduplication.

If existing native notification fields cannot classify actionability without
string heuristics, extend the renderer-safe DTO with explicit fields:

```ts
attentionKind: "action_required" | "review_ready" | "update";
entityKind: "run" | "automation" | "conversation";
entityId: string | null;
```

The host should derive these values from trusted runtime records. The renderer
must not parse opaque payloads or receive previously redacted action data.

### PR 4 — Topbar and contextual Run details

Goal: ship the new information architecture.

Change `AiSidePanel.tsx`:

- replace Tasks and Automations controls with one Work control;
- keep Inbox as the only attention destination;
- display its unresolved attention/review badge;
- rename inspector copy to Run details;
- keep the detail control only on layouts where the rail is hidden;
- ensure opening a Work item can show its explicit run details;
- retain Escape, focus, and overlay dismissal behavior.

Copy changes:

- `Background tasks` -> `Work` or `Agent work`;
- `New task` -> `Delegate work`;
- `Run in background` -> `Delegate`;
- `Automations` -> `Scheduled`;
- `Task inspector` -> `Run details`.

### PR 5 — Consolidate background-job ownership

Goal: remove duplicate fetching and status drift.

Create a workspace-scoped `backgroundJobsStore` or equivalent shared service:

- one bounded background-job fetch per workspace refresh;
- one invalidation path from `altai:agent-inbox-changed`;
- normalized lookup by job ID, chat ID, and automation ID;
- notification and automation selectors join against the shared store;
- stale requests remain protected by request serial/workspace checks.

Then:

- remove `backgroundJobs` ownership from `notificationStore`;
- remove `jobsByAutomationId` fetching from `automationStore`;
- keep notification/ticket mutations in `notificationStore`;
- keep definition mutations in `automationStore`.

This PR is intentionally after the visible IA change so it can be reviewed as
a data-ownership refactor rather than mixed with layout behavior.

### PR 6 — Cleanup, documentation, and telemetry hooks

Remove:

- obsolete peer surface variants;
- duplicate Task/Automation overlay wrappers;
- legacy surface-event normalization after the compatibility window;
- copy referring to background work as a top-level product object.

Update:

- README feature language;
- slash-command descriptions;
- website feature inventory;
- CLI/TUI documentation where it mirrors desktop terminology.

Add local product signals if ALTAI has an approved telemetry mechanism:

- Work opens vs direct Inbox opens;
- Inbox items resolved without opening the referenced run;
- time from `Ready to review` to review action;
- use of Runs vs Scheduled;
- failed deep links or orphaned run references.

Do not add network telemetry as part of this project unless separately
approved.

## 7. File-level impact

Expected primary files:

- `src/modules/ai/components/AiSidePanel.tsx`
- `src/modules/ai/components/TaskRunsPanel.tsx`
- `src/modules/ai/components/AutomationsPanel.tsx`
- `src/modules/ai/components/NotificationInboxPanel.tsx`
- `src/modules/ai/components/AuxiliarySurface.tsx`
- `src/modules/ai/lib/slashCommands.ts`
- `src/modules/ai/lib/agentEventBridge.ts`
- `src/modules/ai/store/agentRunsStore.ts`
- `src/modules/ai/store/automationStore.ts`
- `src/modules/ai/store/notificationStore.ts`
- `src/modules/github/store/assignmentsStore.ts`
- `src/modules/ai/lib/native.ts`

Expected new files:

- `src/modules/ai/components/WorkHubPanel.tsx`
- `src/modules/ai/components/RunsView.tsx`
- `src/modules/ai/components/ScheduledView.tsx`
- `src/modules/ai/lib/workView.ts`
- `src/modules/ai/lib/workView.test.ts`
- optionally `src/modules/ai/store/backgroundJobsStore.ts`
- optionally `src/modules/ai/store/backgroundJobsStore.test.ts`

Backend changes are conditional on explicit Inbox classification/link fields:

- `src-tauri/src/altai/agent/runtime.rs`
- `src-tauri/src/altai/agent/commands.rs`
- related Rust tests.

## 8. Interaction and accessibility requirements

- Work primary tabs use tab semantics and arrow-key navigation.
- Filter controls retain accessible labels and counts.
- Attention badge has screen-reader text and does not rely on color.
- Opening a surface moves focus to its heading or search field.
- Closing returns focus to the control that opened it.
- Deep-linking to a missing/deleted run shows a recoverable empty state rather
  than silently falling back to the active chat.
- Running items use reduced-motion-safe status indicators.
- Narrow and wide layouts expose the same actions even though Run details is
  presented differently.

## 9. Acceptance scenarios

### Manual delegated work

1. User delegates work from the Work surface.
2. One run appears in `Runs > Active`.
3. It does not appear in Inbox while progressing normally.
4. Selecting it opens its transcript or Run details.

### Clarification

1. A run asks a question.
2. The run remains visible in `Runs > Needs attention`.
3. Exactly one Inbox action appears.
4. Replying resumes the run and resolves the Inbox action.
5. The run remains in Work and returns to Active.

### Completion and review

1. A run completes with changes.
2. It appears in `Runs > Ready to review`.
3. One review Inbox item appears when review is required.
4. Marking the notification read does not mark the work reviewed.
5. Applying, dismissing, or explicitly resolving the review clears the review
   item and moves the run to History.

### Failure

1. A run fails.
2. It appears in `Runs > Needs attention`.
3. One Inbox action links to the same run.
4. Retry creates or identifies a new run attempt without duplicating the old
   notification.

### Scheduled work

1. User creates a recurring definition in `Work > Scheduled`.
2. It shows next run and has pause/edit/delete controls.
3. When it fires, a linked execution appears in `Work > Runs`.
4. Normal progress does not enter Inbox.
5. Failure or review-ready completion creates one Inbox event.
6. Opening the definition shows its past runs.

### Responsive inspector

1. Wide layout shows Run details in the right rail.
2. Narrow layout exposes a contextual Run details control.
3. Opening details from a Work row inspects that row's run, not whichever chat
   was previously active.

## 10. Verification

Required checks per PR:

```bash
pnpm test
pnpm build
pnpm lint
```

Focused suites should cover:

- `workView.test.ts`;
- `notificationStore.test.ts`;
- `automationStore.test.ts`;
- `agentRunsStore.test.ts`;
- `slashCommands.test.ts`;
- new Work hub component tests where the repository test setup permits.

Manual desktop QA:

- wide, medium, and narrow AI panel widths;
- active, waiting, failed, ready, completed, and cancelled runs;
- one-time and recurring automation creation;
- clarification reply/resume;
- keyboard-only navigation;
- screen-reader labels for badges and surface controls;
- workspace switch while a refresh is in flight;
- restart with persisted work, tickets, and schedules.

## 11. Rollout and rollback

Ship as small PRs rather than one replacement patch.

During PRs 2-4:

- keep old components available internally;
- route all public commands through the new typed surface router;
- avoid persistence migrations;
- preserve native DTO compatibility unless PR 3 proves explicit fields are
  required.

Rollback boundary:

- the topbar can route Work tabs back to the old panels without touching
  persisted assignments, sessions, notifications, tickets, jobs, or
  automations;
- shared job-store migration must land only after parity tests prove that
  workspace isolation and invalidation behavior remain intact.

## 12. Definition of done

- Topbar no longer exposes Tasks and Automations as peer icons.
- Work contains Runs and Scheduled.
- Inbox contains no normally progressing jobs.
- Inbox badge counts unresolved action/review items, not all running work.
- Run details is contextual and can inspect a run selected from Work.
- Manual and scheduled executions share one normalized run vocabulary.
- Automation definitions link to the runs they created.
- `/tasks`, `/automations`, `/schedule`, `/status`, and `/inbox` remain
  functional.
- One source supplies background-job state to all frontend projections.
- All acceptance scenarios and required checks pass.
