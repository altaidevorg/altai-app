# GitHub Project Management and Agent Workflows

ALTAI turns a local repository into a project-management workspace. Local Git,
todos, the Overview board, and agent tracking work without connecting GitHub.
Connecting GitHub adds remote issues, pull requests, linked Projects, and
publishing actions.

## Prerequisites

For local project management:

1. Open a local Git repository as the ALTAI workspace.
2. Configure an AI model and its API key in **Settings > Models** before
   assigning work to an agent.

To add remote GitHub features:

1. Make sure the repository has a GitHub `origin` remote.
2. Open **Settings > GitHub** and connect your GitHub account.

Linked GitHub Projects require the GitHub `project` scope. If ALTAI asks for
Projects access, reconnect the account. If reconnecting does not request the new
scope, revoke the existing ALTAI authorization in GitHub settings and connect
again.

## Local and Remote Capabilities

GitHub authentication is an optional integration, not an application-wide
login requirement.

| Available without GitHub | Requires a GitHub connection |
| --- | --- |
| Local todo creation and board status changes | Load private repository issues and pull requests |
| Todo, In Progress, Review, and Done workflow | Create, comment on, close, or merge GitHub items |
| Agent assignment, tracking, transcript, and cancellation | Load and synchronize linked GitHub Projects |
| Local status, diff, stage, commit, branch, and worktree operations | Push through GitHub integration and create draft pull requests |
| Repository name derived from an existing origin | Other authenticated remote mutations |

Anonymous read-only loading for public repositories is a planned extension.
The current remote issue, pull request, and Project API paths still require a
GitHub connection.

## Open the GitHub Workspaces

The top of the left sidebar contains three entries:

- **Files** opens the file explorer.
- **GitHub** opens the repository hub: local changes/commits plus pull
  requests and issues when connected.
- **Operations** opens the agent operations workspace (overview, work, runs, inbox).

The badge on **Operations** is an attention count for active agent work
and work that is ready for review.

## Use the GitHub Hub

Select **GitHub** in the sidebar to work with the repository without leaving
ALTAI.

Local changes, diffs, staging, and commits remain available when GitHub is not
connected. The remote section displays a compact connection action instead of
blocking the entire workspace.

After connecting GitHub, you can also:

- Browse and filter pull requests and issues.
- Read descriptions and comments.
- Comment on, close, reopen, or merge supported items.
- Create an issue or pull request.
- Stage and commit local changes.
- Assign an existing issue or pull request to an agent.

### Create an issue

1. Select the **Issues** list.
2. Choose **New issue**.
3. Enter a title and Markdown description.
4. Add one acceptance criterion per line.
5. Select any GitHub labels.
6. Submit the issue.

Acceptance criteria are written to the GitHub issue as a task list, so progress
remains visible to people working outside ALTAI.

### Create and assign an issue in one step

While creating an issue, enable **Create and assign to agent**. Then choose:

- **Agent**: the agent persona that will perform the task.
- **Model**: the model used by the background run.
- **Permissions**: whether the agent asks before changes, edits automatically,
  stays read-only in plan mode, or bypasses approvals when that setting is
  explicitly enabled.
- **Isolated git worktree**: keeps the agent on a dedicated branch outside your
  active working tree. This is enabled by default and is recommended.

After GitHub creates the issue, ALTAI starts an independent background run.

### Assign an existing issue or pull request

Choose **Assign** on an issue or pull request, then:

1. Select the agent, model, and permission mode.
2. Keep **Use an isolated git worktree** enabled for coding work.
3. Add optional instructions such as required tests, implementation constraints,
   or files the agent must not change.
4. Choose **Start background run**.

Each assignment receives its own ALTAI chat session. Starting it does not replace
or interrupt the conversation currently open in the main chat.

## Use Operations

Select **Operations** in the sidebar. The workspace initially opens in **Overview**
mode and combines:

- Local ALTAI todos
- Agent assignments and their run state
- GitHub issues when connected
- GitHub pull requests when connected

Use the source filters to show or hide issues, pull requests, and todos.
Without a GitHub connection, the issue and pull request sources are disabled
while the rest of the board remains interactive.

### Overview columns

The Overview board has four fixed columns:

| Column | Meaning |
| --- | --- |
| **Todo** | Work that has not started |
| **In Progress** | Work being performed, including active agent runs |
| **Review** | Open pull requests and completed agent work awaiting review |
| **Done** | Reviewed or completed work |

Drag a card between columns or open it and use the **Status** selector.

Overview status changes are local to ALTAI. Moving a GitHub issue in Overview
does not close or edit that issue on GitHub. Local todo status changes are also
saved to the todo store.

An active agent assignment automatically places its card in **In Progress**.
When the run finishes, the card moves to **Review**. After reviewing the result,
move it to **Done**.

### Create and assign a local todo

1. Enable the **Todos** source.
2. Choose **New todo**.
3. Enter a short task title and create it.
4. Choose **Assign agent** on the card.

Local todos are useful for work that does not need a public GitHub issue. The
Overview quick-assign action uses the current agent, model, and permission
defaults.

### Run local todos automatically

The **Orchestration** bar turns the local Overview board into a continuous
agent queue:

1. Create one or more local todos in **Todo**.
2. Open **Configure** and adjust the workflow policy when needed.
3. Save the policy to the repository's `WORKFLOW.md`.
4. Choose **Start orchestration**.
5. Keep ALTAI open while the queue is running.

The scheduler claims pending manual todos, creates an isolated Git worktree for
each todo, and starts a background agent run. Agent-generated plan items are not
treated as project work and are never claimed automatically.

Choose **Pause** to stop claiming new todos without cancelling active agents.
Choose **Stop all** to stop the scheduler and request cancellation of its active
runs. A failed dispatch or run is retried in the same worktree with backoff, up
to four attempts.

The todo session used when orchestration starts remains pinned to the queue, so
switching chat sessions does not redirect an active project run. Running and
paused intent is persisted per workspace. When ALTAI restarts, it reconciles
persisted assignments before claiming more work, restores the previous
running/paused state, and continues retry attempt numbering without creating a
duplicate assignment. ALTAI must remain open to execute work; closing it stops
the process until the next launch.

### Configure WORKFLOW.md

The Project Board configuration panel controls:

- Maximum concurrent agents (1–8)
- Maximum attempts per todo (1–10)
- Initial and maximum retry delay
- Per-run permission mode
- Optional model override
- The project-specific agent prompt

Selecting `bypass` in a workflow does not override ALTAI's global safety gate;
**Enable bypass permissions** must still be explicitly enabled in Settings.

Saving the panel creates or updates `WORKFLOW.md` in the repository root:

```md
---
orchestration:
  max_concurrent: 2
  max_attempts: 4
  retry_base_seconds: 5
  retry_max_seconds: 300
agent:
  model_id: null
  permission_mode: ask
---
Complete the assigned local project task end-to-end.

Inspect the repository before editing and run relevant tests.
```

ALTAI checks `WORKFLOW.md` while orchestration is active, so edits made in the
editor apply without restarting the queue. Scheduler limits update immediately;
model, permission, and prompt changes apply to newly dispatched attempts.
Invalid YAML, unknown keys, unsafe ranges, an empty prompt, symlinked files, and
oversized workflow files are rejected. An invalid external edit is shown in the
Project Board while the scheduler continues using the last valid configuration.

When an orchestrated local todo finishes, its card moves to **Review**. Inspect
the transcript and changes, then choose **Apply to workspace**. ALTAI commits
remaining changes inside the agent worktree and cherry-picks the worktree
commits onto the current workspace branch. The target workspace must be clean.
If Git reports a conflict, ALTAI aborts the cherry-pick and leaves the target
unchanged. A successful apply marks the todo **Done**.

## Use a Linked GitHub Project

The board selector next to the repository name lists GitHub Projects linked to
the current repository. Choose one to open its real GitHub Project board.

A linked Project behaves differently from Overview:

- Its columns come from the Project's single-select **Status** field.
- Dragging a card updates that Status field on GitHub.
- Changing Status in the card details also updates GitHub.
- ALTAI updates the card immediately and restores its previous status if the
  GitHub request fails.

If an empty Project is selected, choose **Add open issues & PRs** to add the
repository's current open items. If the Project has no single-select Status
field, add one on GitHub before using the board.

ALTAI currently loads up to the first 300 Project items.

## Inspect a Card

Select a board card to open its details drawer. Depending on the card and
assignment state, the drawer provides:

- Title, source, author, number, and description
- Current board status
- Assigned agent, model, run state, and branch
- **Open transcript** to switch to the assignment's chat session
- **Cancel** for an active run
- **Apply to workspace** for completed orchestrated local todo work
- **Create draft PR** or **Retry draft PR** for completed isolated issue work
- **Open PR** after a draft pull request has been created
- **Open on GitHub** for remote items

GitHub Project draft notes cannot be assigned directly. Convert a draft note to
an issue on GitHub first.

## Review and Publish Agent Work

For an issue assigned with an isolated worktree:

1. The agent works in a dedicated worktree and branch.
2. Follow progress from the assignment rail or choose **Open transcript**.
3. Approve requested actions when using **Ask before changes**.
4. When the run completes, inspect its changes and test results.
5. Choose **Create draft PR**.

ALTAI stages remaining changes in the isolated worktree, creates a commit when
needed, publishes the branch, and opens a draft pull request that closes the
original issue. If an open pull request already exists for that branch, ALTAI
reuses it instead of creating a duplicate.

Publishing is always an explicit user action. A completed agent run does not
automatically push a branch or open a pull request.

## Manage Running Assignments

The assignment rail at the top of Overview shows current and previous runs. Each
assignment includes its state, token count when available, and isolated branch
name.

Available actions include:

- Open the assignment session.
- Cancel an active run.
- Publish or open its draft pull request.
- Remove an assignment from the rail.

Removing an active assignment first requests cancellation so the background run
is not left orphaned.

## Troubleshooting

### "Connect your GitHub account"

This message applies only to the remote section. Local Git, todos, Overview, and
agent tracking remain available. Open **Settings > GitHub** when you want to
enable remote GitHub data and actions.

### "This repository has no GitHub remote (origin)"

Add or correct the repository's GitHub `origin` remote, then reopen or refresh
the workspace.

### "Projects access needed"

Reconnect GitHub to grant the `project` scope. If GitHub does not show a new
authorization prompt, revoke ALTAI under
[GitHub application settings](https://github.com/settings/applications) and
connect again.

### A linked Project does not appear

Confirm that the Project is linked to the current repository and that the
connected account can access it. Then refresh the board.

### A linked Project cannot be displayed as a board

Add a single-select field named **Status** to the Project on GitHub.

### An agent run cannot start

Check that:

- A local Git workspace is open.
- The selected model has a valid API key.
- The repository is a valid Git repository.
- The worktree can be created from the current branch.

### A status change returns to its previous column

For linked GitHub Projects, ALTAI rolls back optimistic changes when GitHub
rejects the update. Check the error shown above the board, repository access,
Project permissions, and the GitHub connection.
