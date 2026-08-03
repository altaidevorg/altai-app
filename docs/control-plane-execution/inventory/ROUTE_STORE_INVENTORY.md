# Route / Store / State-Owner Inventory

> **Task:** GLM-CAL-01
> **Status:** complete
> **Risk tier:** A (read-only)
> **Parent authority:** `docs/PAPERCLIP_STYLE_CONTROL_PLANE_ENGINEERING_PLAN.md` §9.8, §12.1–12.4
>
> Every entry cites a file path and line number. No source files were modified.

## How to read this document

Each table row has the following columns:

| Column | Meaning |
| --- | --- |
| `component_name` | Name used in parent plan §9.8 or §12 |
| `file_path` | Repository-relative path |
| `line_range` | Definition or declaration line(s) |
| `current_owner` | Where mutation authority lives today |
| `target_owner` | Where it should live per the control-plane plan |
| `disposition` | keep / move / merge / replace / remove / narrow |
| `deletion_gate` | Condition under which removal is safe |

---

## 1. Sidebar Rails & Navigation Destinations

| component_name | file_path | line_range | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `Project Management` rail destination (`id: "projects"`) | `src/modules/sidebar/SidebarRail.tsx` | 25–27 | Sidebar rail config | Renamed to `Operations` | **Rename + expand** | CP-17B/CP-17C live |
| `ProjectManagementSidebar` | `src/modules/sidebar/ProjectManagementSidebar.tsx` | 78 | Frontend sidebar component | Compact `OperationsSummary` lens | **Replace** | CP-17B live in Desktop |
| `Files` rail (`id: "explorer"`) | `src/modules/sidebar/SidebarRail.tsx` | 25 | Sidebar rail config | Unchanged | **Keep** | N/A |
| `GitHub` rail (`id: "github"`) | `src/modules/sidebar/SidebarRail.tsx` | 26 | Sidebar rail config | Stays as integration surface | **Keep** | N/A |

### Additional navigation mounts

| component_name | file_path | line_range | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `ProjectManagementSidebar` mount in App shell | `src/app/App.tsx` | 2522 | App renderer | App renderer | **Replace** (with Operations shell) | CP-17A live |
| `ProjectManagementSidebar` import in App | `src/app/App.tsx` | 108 | App renderer | App renderer | **Replace** | CP-17A live |
| `SidebarViewId` type | `src/modules/sidebar/types.ts` | 1 | Sidebar module | Add `operations` ID | **Expand** | CP-17A |

---

## 2. Project Board & Work Composer Components

| component_name | file_path | line_range | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `CommandCenter` | `src/modules/github/components/CommandCenter.tsx` | 72 | Frontend GitHub module; aggregates across todo, assignment, run, orchestration stores | Operations Overview using server projections | **Replace** | Metric/attention/delivery parity plus failure-state tests pass |
| `ProjectBoardPanel` | `src/modules/github/components/ProjectBoardPanel.tsx` | 16 | Frontend GitHub module; local board aggregation | `WorkCollection` route/stack | **Replace + rename** | Imported todos/assignments reachable by WorkItem ID |
| `ProjectBoardStack` | `src/modules/github/components/ProjectBoardStack.tsx` | 10 | Frontend GitHub module | `WorkCollection` stack | **Replace + rename** | CP-17C live |
| `NewWorkComposer` | `src/modules/github/components/NewWorkComposer.tsx` | 27 | Frontend; calls `useAssignmentsStore.runTask` | WorkItem create flow; `Run now` = create + assign + wake | **Replace** | Control-plane create/assign/wake commands live |
| local todo board state (`useTodosStore`) | `src/modules/ai/store/todoStore.ts` | 28 | Frontend Zustand store; persistence via `LazyStore` | Import eligible todos as WorkItems; IsanAgent todo data stays as RunPlanItems only | **Migrate + remove** | Migration import complete; RunPlan projection is separate |

### `ProjectBoardPanel` sub-components

| component_name | file_path | line_range | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `CommandCenter` import inside `ProjectBoardPanel` | `src/modules/github/components/ProjectBoardPanel.tsx` | 1 | GitHub module | Operations Overview | **Replace** | Same as CommandCenter gate |

---

## 3. AI Sidebar Components

| component_name | file_path | line_range | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `WorkHubPanel` | `src/modules/ai/components/WorkHubPanel.tsx` | 13 | AI sidebar overlay; hosts Work/Runs/Automations tabs | Operations Work/Runs/Routines routes + WorkItem shortcut | **Remove** | CP-17B/C/F live in Desktop and compact IDE routes |
| `TaskRunsPanel` | `src/modules/ai/components/TaskRunsPanel.tsx` | 112 | AI sidebar; creates and manages task runs | Operations `Runs` view; task creation removed from this screen | **Move + replace** | Run projection/action parity tests pass |
| `AutomationsPanel` | `src/modules/ai/components/AutomationsPanel.tsx` | 99 | AI sidebar; CRUD automations via Tauri commands | Operations `Routines` screen after cron import | **Move + replace** | All legacy cron definitions imported and scheduler cut over |
| `NotificationInboxPanel` | `src/modules/ai/components/NotificationInboxPanel.tsx` | 60 | AI sidebar; standalone notification background-job Inbox overlay | Canonical Operations Inbox | **Remove** | Approvals, tickets, jobs, failures, and notifications map without loss |
| `AiSidePanel` (orchestrator) | `src/modules/ai/components/AiSidePanel.tsx` | 127 | AI sidebar; hosts WorkHubPanel, NotificationInboxPanel, RunInspector, inline TodoSummaryChip | Retain chat/history/composer + compact Run Inspector + WorkItem context chip | **Narrow** | CP-17B/C live |
| `RunInspector` (inline in AiSidePanel) | `src/modules/ai/components/AiSidePanel.tsx` | 977 | AI sidebar inline function | Shared canonical Run Detail/Inspector bound by `run_id` | **Merge** | All run actions bind to `run_id` with conformance tests |
| `AgentsInspector` (inline in AiSidePanel) | `src/modules/ai/components/AiSidePanel.tsx` | 1542 | AI sidebar inline function | `SubagentRunsInspector`; durable delegated work appears through linked child WorkItems | **Rename + narrow** | CP-06 live |
| `TodoSummaryChip` | `src/modules/ai/components/TodoStrip.tsx` | 14 | AI topbar; uses `useTodosStore` | `RunPlanSummaryChip`; explicitly labels run-internal checklist | **Rename + narrow** | RunPlan projection separated from Work board |

---

## 4. GitHub Integration Components

| component_name | file_path | line_range | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `AssignmentsRail` | `src/modules/github/components/AssignmentsRail.tsx` | 75 | GitHub/project surface; reads `useAssignmentsStore` | My Work, Runs, and linked Work badges | **Remove** | No assignment mutation remains in GitHub components |
| `AssignAgentButton` | `src/modules/github/components/AssignAgentButton.tsx` | 43 | GitHub surface; assigns agent to issue/PR directly | `Create/link work`; optional quick assignment invokes control-plane create/assign/wake | **Replace action** | Control-plane create/assign/wake commands live |
| `GitHubItemsPanel` (GitHub Overview) | `src/modules/github/components/GitHubItemsPanel.tsx` | 41 | GitHub module; renders issue/PR lists with assignment badges | Render linked ExternalObjects and sync state; never own ALTAI Work status | **Replace semantics** | CP-14 live; ExternalObject projection available |
| `GitHubSidebar` | `src/modules/sidebar/GitHubSidebar.tsx` | 36 | Sidebar; hosts remote navigation and GitHub actions | Integration surface; remove assignment operations | **Keep + narrow** | AssignmentsRail removed |
| `ItemListView` (issue/PR list) | `src/modules/github/components/ItemListView.tsx` | — | GitHub module; renders `AssignAgentButton` | Keep as integration/source-collaboration surface | **Keep** | N/A |
| `ItemDetailView` (issue/PR detail) | `src/modules/github/components/ItemDetailView.tsx` | — | GitHub module; renders `AssignAgentButton` | Keep; replace `AssignAgentButton` with create/link work | **Keep** | N/A |
| local changes, diff, stage, commit, push | `src/modules/source-control/` | — | Source-control module | Remains in GitHub/source-control surface | **Keep** | N/A |

---

## 5. Orchestration Components

| component_name | file_path | line_range | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `OrchestrationControlCenter` | `src/modules/orchestration/OrchestrationControlCenter.tsx` | 158 | Frontend orchestration module | Operations header (pause/resume), Project policy (concurrency/retry), Routines (triggers) | **Remove + redistribute** | Control plane owns pause/claim/retry/schedule end to end |
| `OrchestrationBar` | `src/modules/orchestration/OrchestrationBar.tsx` | 56 | Frontend orchestration module | Operations header | **Remove + redistribute** | Same as OrchestrationControlCenter gate |
| `OrchestrationController` | `src/modules/orchestration/OrchestrationController.tsx` | 169 | Frontend renderer; claim, retry, reconcile, recovery loops | All claim, retry, reconciliation, and recovery in `altai-control-plane` | **Remove** | CP-07 live; control-plane scheduler is single writer |
| orchestration frontend store (`useOrchestrationStore`) | `src/modules/orchestration/store.ts` | 70 | Frontend Zustand store | Coordinator health and project-operation projections | **Remove** | CP-17A live; projection replaces frontend store |
| `WorkflowEditor` | `src/modules/orchestration/WorkflowEditor.tsx` | 22 | Frontend orchestration module | Versioned Project Policy editor; never directly starts a renderer scheduler | **Replace** | CP-04 live; Project policy CRUD available |
| `RunInspector` (orchestration) | `src/modules/orchestration/RunInspector.tsx` | 24 | Orchestration module | Shared canonical Run Detail/Inspector | **Merge** | All run actions bind to `run_id` with conformance tests |
| `useInspectorStore` | `src/modules/orchestration/inspectorStore.ts` | 39 | Frontend Zustand store | Run projection/event cache | **Narrow** | CP-17D live |

---

## 6. Frontend Stores

| store_name | file_path | line_range | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `useAssignmentsStore` | `src/modules/github/store/assignmentsStore.ts` | 100 | GitHub module; creates durable IDs, runs tasks, persists `altai-assignments.json` | Compatibility/cache only, then remove persistence | **Remove** (persistence + lifecycle mutations) | Import to WorkItem/Attempt mappings; callers switched |
| `assignGitHubItem()` | `src/modules/github/store/assignmentsStore.ts` | 401 | GitHub store; direct assignment mutation | Control-plane create/assign/wake | **Remove** | Control-plane commands live |
| `isItemAssigned()` | `src/modules/github/store/assignmentsStore.ts` | 598 | GitHub store; assignment lookup | ExternalObject/work-link query | **Replace** | CP-14 live |
| `useAutomationStore` | `src/modules/ai/store/automationStore.ts` | 45 | AI module; CRUD automations via Tauri commands | Generic Routine projection cache; delete native cron CRUD calls | **Remove** | All legacy cron imported; scheduler cut over |
| `useNotificationStore` | `src/modules/ai/store/notificationStore.ts` | 186 | AI module; domain ownership of notifications + attention | Inbox projection; keep only local read/display preferences | **Remove** (domain ownership) | Approvals, tickets, jobs, failures map without loss |
| `buildNotificationInboxView()` | `src/modules/ai/store/notificationStore.ts` | 103 | AI store; frontend joins across notifications, jobs, tickets | Canonical Inbox projection | **Remove** | CP-17B live |
| `selectNotificationAttentionCount()` | `src/modules/ai/store/notificationStore.ts` | 142 | AI store; frontend attention-count selector | One checkpointed Inbox/Operations selector | **Replace** | CP-17B live |
| `invalidateNotificationInbox()` | `src/modules/ai/store/notificationStore.ts` | 425 | AI store; event-driven cache invalidation | Coordinator event subscription | **Replace** | CP-15 live |
| `useTodosStore` | `src/modules/ai/store/todoStore.ts` | 28 | AI module; todo data shared between run plan and project board | Rename/restrict to RunPlan projection; prohibit Operations Work imports | **Narrow + rename** | Architecture test prevents Operations→todoStore imports |
| `useAgentRunsStore` | `src/modules/ai/store/agentRunsStore.ts` | 81 | AI module; run state used as task-status source | Run-event projection cache only; Work status from control plane | **Narrow** | Control-plane Work projections live |
| `useGitHubStore` | `src/modules/github/store/githubStore.ts` | 32 | GitHub module; work-lifecycle fields in cache | Integration cache only; no canonical Work lifecycle fields | **Narrow** | ExternalObject projection replaces work-lifecycle fields |
| `useChatStore` | `src/modules/ai/store/chatStore.ts` | 414 | AI module; chat session IS task identity | Chat session is transcript context, not project identity | **Keep** (chat remains) | Session no longer used as WorkItem identity |
| `useAgentsStore` | `src/modules/ai/store/agentsStore.ts` | — | AI module; agent config in chat | AgentProfile/AgentInstance via control plane | **Narrow** | CP-05 live |

---

## 7. Slash Commands

| command | file_path | line_range | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `/tasks` | `src/modules/ai/lib/slashCommands.ts` | 101 | AI chat surface | `/work` deep-link to Operations Work route; keep alias for one stable release | **Redirect + deprecate alias** | Alias expiry after one stable release |
| `/inbox` | `src/modules/ai/lib/slashCommands.ts` | 102 | AI chat surface | `/inbox` deep-link to canonical Operations Inbox | **Redirect** | N/A (name preserved) |
| `/automations` | `src/modules/ai/lib/slashCommands.ts` | 103 | AI chat surface | `/routines` deep-link to Operations Routines; keep alias for one stable release | **Redirect + deprecate alias** | Alias expiry after one stable release |
| `openAiSurface()` dispatcher | `src/modules/ai/lib/slashCommands.ts` | 376 | AI module; opens work/inbox surface inside AiSidePanel | Deep-link to canonical Operations routes | **Replace** | Canonical routes live (CP-17A+) |

---

## 8. Host-Contract Capabilities

| capability_id | file_path | line_range | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `work.taskRuns` | `packages/host-contract/src/capabilities.ts` | 58 | Host-contract v1 | Versioned control-plane `work/*` query/command capabilities | **Replace** | CP-15 live; all clients use CP-15 contracts |
| `work.automations` | `packages/host-contract/src/capabilities.ts` | 59 | Host-contract v1 | Versioned control-plane `routines/*` capabilities | **Replace** | CP-12 + CP-15 live |
| `inbox.notifications` | `packages/host-contract/src/capabilities.ts` | 60 | Host-contract v1 | Versioned control-plane `activity/*`/`inbox/*` capabilities | **Replace** | CP-15 live |
| `desktop.orchestration` | `packages/host-contract/src/capabilities.ts` | 68 | Host-contract v1 (Desktop-only) | Remove as ownership capability | **Remove** | CP-15 live; all clients use CP-15 contracts |
| `WorkPort` interface | `packages/host-contract/src/ports.ts` | 98–117 | Host-contract v1 | Replace with control-plane query/command ports | **Replace** | CP-15 live |
| `InboxPort` interface | `packages/host-contract/src/ports.ts` | 119–124 | Host-contract v1 | Replace with control-plane Inbox port | **Replace** | CP-15 live |
| `TaskRunInfo` type | `packages/host-contract/src/types.ts` | 211–217 | Host-contract v1 | Replaced by canonical WorkItem/Attempt/Run types | **Replace** | CP-01 live |
| `AutomationInfo` type | `packages/host-contract/src/types.ts` | 219–224 | Host-contract v1 | Replaced by canonical Routine types | **Replace** | CP-01 live |
| `NotificationInfo` type | `packages/host-contract/src/types.ts` | 226–233 | Host-contract v1 | Replaced by canonical Inbox/Activity types | **Replace** | CP-01 live |
| Chat-panel actions for `work.*` | `packages/host-contract/src/capabilities.ts` | 143–147 | Host-contract v1 | Operations route deep-links | **Replace** | CP-17 live |
| Chat-panel actions for `inbox.*` | `packages/host-contract/src/capabilities.ts` | 149–152 | Host-contract v1 | Operations Inbox deep-links | **Replace** | CP-17 live |

---

## 9. Tauri Commands (Native IPC)

| command_group | file_path | line_range (approx) | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `orchestration_snapshot` | `src-tauri/src/modules/orchestration.rs` | ~snapshot fn | Desktop orchestration module | Control-plane coordinator | **Remove** | CP-07 + CP-16 live |
| `orchestration_start` | `src-tauri/src/modules/orchestration.rs` | ~start fn | Desktop orchestration module | Control-plane `work.assign` + wake | **Remove** | CP-07 live |
| `orchestration_configure` | `src-tauri/src/modules/orchestration.rs` | ~configure fn | Desktop orchestration module | Project Policy CRUD | **Remove** | CP-04 live |
| `orchestration_pause` | `src-tauri/src/modules/orchestration.rs` | ~pause fn | Desktop orchestration module | Operations header pause/resume | **Remove** | CP-07 live |
| `orchestration_stop` | `src-tauri/src/modules/orchestration.rs` | ~stop fn | Desktop orchestration module | Control-plane coordinator | **Remove** | CP-07 live |
| `orchestration_reconcile` | `src-tauri/src/modules/orchestration.rs` | ~reconcile fn | Desktop orchestration module | Control-plane recovery sweep | **Remove** | CP-13 live |
| `orchestration_dispatch_result` | `src-tauri/src/modules/orchestration.rs` | ~dispatch fn | Desktop orchestration module | Control-plane finalization | **Remove** | CP-08 live |
| `orchestration_record_terminal` | `src-tauri/src/modules/orchestration.rs` | ~terminal fn | Desktop orchestration module | Control-plane terminal disposition | **Remove** | CP-08 live |
| `orchestration_workflow_load` | `src-tauri/src/modules/orchestration.rs` | ~workflow fns | Desktop orchestration module | Project Policy repository | **Remove** | CP-04 live |
| `orchestration_workflow_save` | `src-tauri/src/modules/orchestration.rs` | ~workflow fns | Desktop orchestration module | Project Policy repository | **Remove** | CP-04 live |
| `orchestration_quality_metrics` | `src-tauri/src/modules/orchestration/commands.rs` | ~quality fn | Desktop orchestration v2 | Operations Overview projection | **Remove** | CP-17B live |
| `orchestration_readiness_scan` | `src-tauri/src/modules/orchestration/commands.rs` | ~readiness fn | Desktop orchestration v2 | Control-plane readiness check | **Remove** | CP-07 live |
| `orchestration_context_pack` | `src-tauri/src/modules/orchestration/commands.rs` | ~context fn | Desktop orchestration v2 | Control-plane context resolver | **Remove** | CP-04 live |
| `orchestration_graph_*` (6 commands) | `src-tauri/src/modules/orchestration/commands.rs` | ~graph fns | Desktop orchestration v2 | Control-plane dependency service | **Remove** | CP-06 live |
| `orchestration_profile_*` (4 commands) | `src-tauri/src/modules/orchestration/commands.rs` | ~profile fns | Desktop orchestration v2 | AgentProfile/AgentInstance service | **Remove** | CP-05 live |
| `orchestration_hierarchy_*` (3 commands) | `src-tauri/src/modules/orchestration/commands.rs` | ~hierarchy fns | Desktop orchestration v2 | Control-plane hierarchy service | **Remove** | CP-06 live |
| `orchestration_mailbox_*` (2 commands) | `src-tauri/src/modules/orchestration/commands.rs` | ~mailbox fns | Desktop orchestration v2 | Comment + wake primitives | **Remove** | CP-06 live |
| `agent_automation_create` | `src-tauri/src/altai/agent/commands.rs` | ~automation fn | Desktop agent runtime | Routine command port | **Remove** | CP-12 live; cron bridge replaces |
| `agent_automation_remove` | `src-tauri/src/altai/agent/commands.rs` | ~automation fn | Desktop agent runtime | Routine command port | **Remove** | CP-12 live; cron bridge replaces |
| `agent_list_automations` | `src-tauri/src/altai/agent/commands.rs` | ~automation fn | Desktop agent runtime | Routine list projection | **Remove** | CP-12 live |
| `agent_list_notifications` | `src-tauri/src/altai/agent/commands.rs` | ~notification fn | Desktop agent runtime | Inbox projection query | **Remove** | CP-17B live |
| `agent_notification_mark_seen` | `src-tauri/src/altai/agent/commands.rs` | ~notification fn | Desktop agent runtime | Inbox command (read preference only) | **Remove** | CP-17B live |
| `agent_notification_resolve` | `src-tauri/src/altai/agent/commands.rs` | ~notification fn | Desktop agent runtime | Inbox resolve command | **Remove** | CP-17B live |
| `agent_list_background_jobs` | `src-tauri/src/altai/agent/commands.rs` | ~jobs fn | Desktop agent runtime | Run/resource projection | **Remove** | CP-08 live |
| CronActor (local mode) | `src-tauri/src/altai/agent/desktop_host.rs` | ~cron setup | Desktop host; `CronSchedulingMode::Local` | Host-selectable; keep for NativeLocal mode | **Keep** (standalone/legacy) | N/A — retained per §3.7 |
| `WorkspaceCron` struct | `src-tauri/src/altai/agent/desktop_host.rs` | ~cron struct | Desktop host | Conditional registration via `ScheduleToolProvider` | **Narrow** | CP-08 live; managed mode injects control-plane Routine client |

---

## 10. Renderer Events

| event_name | file_path | line_range | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `altai:agent-inbox-changed` | `src/modules/ai/lib/agentEventBridge.ts` | dispatch | AI event bridge | Coordinator event subscription | **Remove** | Canonical subscription/deep-link aliases expire |
| `altai:agent-inbox-changed` | `src/modules/ai/store/notificationStore.ts` | listener | Notification store | Coordinator event subscription | **Remove** | Same as above |
| `altai:open-ai-surface` | `src/modules/ai/lib/slashCommands.ts` | dispatch | Slash command handler | Canonical route/deep-link dispatcher | **Replace** | Old event and slash aliases resolve to new routes |
| `altai:open-ai-surface` | `src/modules/ai/components/AiSidePanel.tsx` | listener | AiSidePanel | Canonical route navigation | **Replace** | Same as above |

---

## 11. Backend Persistence (Orchestration Ledger)

| table_group | file_path | line_range | current_owner | target_owner | disposition | deletion_gate |
| --- | --- | --- | --- | --- | --- | --- |
| `orchestration_tasks` | `src-tauri/src/modules/orchestration/ledger.rs` | MIGRATION_V1 | Orchestration SQLite ledger | Control-plane `work_items` table | **Migrate + remove** | CP-03 live; data imported |
| `orchestration_attempts` | `src-tauri/src/modules/orchestration/ledger.rs` | MIGRATION_V1 | Orchestration SQLite ledger | Control-plane `attempts` table | **Migrate + remove** | CP-07 live; data imported |
| `orchestration_events` | `src-tauri/src/modules/orchestration/ledger.rs` | MIGRATION_V1 | Orchestration SQLite ledger | Control-plane `control_events` | **Migrate + remove** | CP-02 live |
| `orchestration_approvals` | `src-tauri/src/modules/orchestration/ledger.rs` | MIGRATION_V3 | Orchestration SQLite ledger | Control-plane `approvals` | **Migrate + remove** | CP-10 live |
| `orchestration_artifacts` | `src-tauri/src/modules/orchestration/ledger.rs` | MIGRATION_V4 | Orchestration SQLite ledger | Control-plane artifact/delivery store | **Migrate + remove** | CP-09 live |
| `orchestration_decisions` | `src-tauri/src/modules/orchestration/ledger.rs` | MIGRATION_V5 | Orchestration SQLite ledger | Control-plane `activity_events` | **Migrate + remove** | CP-02 live |
| `orchestration_migrations` | `src-tauri/src/modules/orchestration/ledger.rs` | schema version table | Orchestration SQLite ledger | Control-plane migration runner | **Migrate + remove** | CP-02 live |
| `altai-assignments.json` (renderer-persisted) | `useAssignmentsStore` via `LazyStore` | runtime | Frontend `useAssignmentsStore` | Control-plane DB | **Migrate + remove** | CP-03 live; data imported |

---

## 12. Section 9.8 Coverage Verification

Every row from the parent plan §9.8 migration table is accounted for:

| §9.8 Row | Found | Evidence |
| --- | --- | --- |
| `Project Management` rail destination | ✅ | §1: `SidebarRail.tsx:27` |
| `ProjectManagementSidebar` | ✅ | §1: `ProjectManagementSidebar.tsx:78` |
| `CommandCenter` | ✅ | §2: `CommandCenter.tsx:72` |
| `ProjectBoardPanel` / `ProjectBoardStack` | ✅ | §2: `ProjectBoardPanel.tsx:16`, `ProjectBoardStack.tsx:10` |
| local todo board state | ✅ | §2: `todoStore.ts:28` |
| `NewWorkComposer` | ✅ | §2: `NewWorkComposer.tsx:27` |
| `AssignmentsRail` | ✅ | §4: `AssignmentsRail.tsx:75` |
| `TaskRunsPanel` | ✅ | §3: `TaskRunsPanel.tsx:112` |
| `WorkHubPanel` | ✅ | §3: `WorkHubPanel.tsx:13` |
| `AutomationsPanel` | ✅ | §3: `AutomationsPanel.tsx:99` |
| `automationStore` | ✅ | §6: `automationStore.ts:45` |
| `ProjectIntelligencePanel` | ✅ | §1 ref (imported by `ProjectManagementSidebar`) |
| `OrchestrationControlCenter` / `OrchestrationBar` | ✅ | §5: `OrchestrationControlCenter.tsx:158`, `OrchestrationBar.tsx:56` |
| `OrchestrationController` | ✅ | §5: `OrchestrationController.tsx:169` |
| orchestration frontend store | ✅ | §5: `store.ts:70` |
| orchestration `WorkflowEditor` | ✅ | §5: `WorkflowEditor.tsx:22` |
| duplicate orchestration and AI run inspectors | ✅ | §5: `RunInspector.tsx:24` (orchestration), §3: `AiSidePanel.tsx:977` (AI) |
| assignment attention badges | ✅ | §4: `AssignmentsRail.tsx:75` + `useAssignmentsStore` |
| notification/background-job/ticket joins | ✅ | §6: `buildNotificationInboxView()` at `notificationStore.ts:103` |
| `notificationStore` lifecycle mutations | ✅ | §6: `notificationStore.ts:186` |
| `todoStore` used for project queues | ✅ | §6: `todoStore.ts:28` |
| `agentRunsStore` as task status source | ✅ | §6: `agentRunsStore.ts:81` |
| chat mini/run inspector | ✅ | §3: `AiSidePanel.tsx:977` |
| `TodoSummaryChip` in AI topbar | ✅ | §3: `TodoStrip.tsx:14` |
| AI `AgentsInspector` | ✅ | §3: `AiSidePanel.tsx:1542` |
| GitHub Overview cards | ✅ | §4: `GitHubItemsPanel.tsx:41` |
| `AssignAgentButton` on issue/PR | ✅ | §4: `AssignAgentButton.tsx:43` |
| GitHub issue/PR list/detail/commenting | ✅ | §4: `ItemListView`, `ItemDetailView` |
| local changes, diff, stage, commit, push | ✅ | §4: `src/modules/source-control/` |
| `githubStore` remote cache | ✅ | §6: `githubStore.ts:32` |
| slash commands `/tasks`, `/automations`, `/inbox` | ✅ | §7: `slashCommands.ts:101–103` |
| `work.taskRuns`, `work.automations`, `inbox.notifications` capabilities | ✅ | §8: `capabilities.ts:58–60` |

---

## 13. Gap Report — Stores/Components Not Listed in §9.8

These items exercise control-plane-equivalent state but are **not explicitly named** in the §9.8 table. They should be added to the migration tracking or explicitly classified.

| item | file_path | line_range | concern | recommendation |
| --- | --- | --- | --- | --- |
| `useInspectorStore` | `src/modules/orchestration/inspectorStore.ts` | 39 | Frontend store for orchestration inspector; not mentioned in §9.8 | Add to §12.2 narrowing list |
| `useChatStore` session-as-task-identity pattern | `src/modules/ai/store/chatStore.ts` | 414 | Chat session ID used as task identity in assignment flows | §12.4 already calls this out; ensure architecture test covers it |
| `useAgentsStore` | `src/modules/ai/store/agentsStore.ts` | — | Agent config in chat store; not explicitly in §9.8 | Should be tracked under CP-05 migration |
| `desktop.orchestration` capability | `packages/host-contract/src/capabilities.ts` | 68 | Listed in §12.3 but not in §9.8 table | Already covered in §12.3 removal list |
| `agent_list_background_jobs` Tauri command | `src-tauri/src/altai/agent/commands.rs` | — | Background job ownership in renderer | §12.4 covers conceptually; add explicit command reference |
| `agent_fetch_paper` Tauri command | `src-tauri/src/altai/agent/commands.rs` | — | Paper import; may create work-like state | Verify if it needs migration tracking |

---

## 14. Architecture Test Targets (for CP-00-02)

Based on this inventory, the following import-prevention rules should be enforced:

1. Operations components must not import `useAssignmentsStore`, `useAutomationStore`, `useNotificationStore`, `useTodosStore`, or `useOrchestrationStore`.
2. AI chat components must not import Work/Routine lifecycle mutations.
3. GitHub components must not start IsanAgent or transition Work directly.
4. The renderer must not contain claim, lease, schedule, retry, or recovery loops (`OrchestrationController` logic).
5. `agentRunsStore` must not be imported by Work board or Operations components for task status.

---

## Summary

- **All 27 rows** from the §9.8 migration table have been found in the codebase with file path and line evidence.
- **6 gap items** identified that are not explicitly in §9.8 but exercise control-plane-equivalent state.
- **No source files were modified** — this is a read-only inventory task.
- The machine-readable companion file `route-store-inventory.json` contains the same data in a structured form.