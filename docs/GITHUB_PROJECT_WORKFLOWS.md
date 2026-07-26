# GitHub Project Management and Agent Workflows

ALTAI can turn a local GitHub repository into a project-management workspace.
You can browse issues and pull requests, organize work on a board, assign coding
tasks to background agents, review their progress, and publish completed issue
work as a draft pull request.

## Prerequisites

Before using the GitHub features:

1. Open a local Git repository as the ALTAI workspace.
2. Make sure the repository has a GitHub `origin` remote.
3. Open **Settings > GitHub** and connect your GitHub account.
4. Configure an AI model and its API key in **Settings > Models**.

Linked GitHub Projects require the GitHub `project` scope. If ALTAI asks for
Projects access, reconnect the account. If reconnecting does not request the new
scope, revoke the existing ALTAI authorization in GitHub settings and connect
again.

## Open the GitHub Workspaces

The top of the left sidebar contains four entries:

- **Files** opens the file explorer.
- **Git** opens source control.
- **GitHub** opens the repository's pull request and issue hub.
- **Projects** opens the project-management board.

The badge on **Projects** is an attention count for active agent work and work
that is ready for review.

## Use the GitHub Hub

Select **GitHub** in the sidebar to work with the repository without leaving
ALTAI.

From this view you can:

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

## Use the Project Board

Select **Projects** in the sidebar. The board initially opens in **Overview**
mode and combines:

- GitHub issues
- GitHub pull requests
- Local ALTAI todos
- Agent assignments and their run state

Use the source filters to show or hide issues, pull requests, and todos.

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

Open **Settings > GitHub** and complete the connection flow.

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
