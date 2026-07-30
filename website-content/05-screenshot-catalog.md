# Private Screenshot Reference Catalog

All images were captured from the running ALTAI desktop app on 2026-07-29.
Original files are 2704 × 1698 PNGs at 2× scale.

**Do not use these files as public website artwork.** They exist only to help
the implementation team compare spacing, proportions, hierarchy, typography,
and control placement while building the interactive HTML/CSS replica.

## Reference rules

- Keep the source originals unchanged.
- Do not import these images into public page components.
- Do not generate WebP/AVIF website derivatives.
- Preserve the originals for visual regression review.
- Do not trace private workspace names, usernames, or tool-specific folders
  into the replica.
- Use the screenshots to compare, then implement with semantic components.

---

## 1. `altai-agent-workspace.png`

**Shows**

- file explorer;
- native editor workspace;
- AI sidebar;
- Dataset Generator persona;
- task starters;
- multi-line composer;
- model and agent selection;
- local-first desktop layout.

**Best placement**

- hero;
- ML/data introduction;
- product overview.

**Recommended crop**

- desktop: crop out the file explorer for public use unless the demo workspace
  has been cleaned of unrelated tool-specific folders;
- tablet: retain the central workspace and full assistant;
- mobile: crop to assistant header, task starters, and composer.

**Caption**

Dataset Generator inside the local ALTAI workspace.

**Alt text**

ALTAI desktop workspace with a file explorer on the left and the Dataset
Generator agent on the right, offering starter tasks for synthetic Q&A,
labelled intents, and evaluation data.

---

## 2. `altai-agent-terminal.png`

**Shows**

- file explorer;
- editor workspace;
- true integrated terminal;
- assistant beside the terminal;
- Dataset Generator persona and model controls.

**Best placement**

- agentic coding;
- ML execution;
- “real IDE surfaces” section.

**Recommended crop**

Keep the terminal prompt, assistant identity, and enough of the file explorer
to establish that this is a real workspace. Before public use, re-capture with
a neutral terminal prompt so the local username and machine name are not
published.

**Caption**

Agent, workspace, and PTY terminal in one native window.

**Alt text**

ALTAI desktop app showing the project file tree, an integrated shell terminal,
and the Dataset Generator agent side by side.

---

## 3. `altai-github-agent.png`

**Shows**

- native GitHub sidebar;
- repository identity;
- pull-request and issue entry points;
- branch selector;
- commit graph;
- commit and push actions;
- assistant open in the same workspace.

**Best placement**

- Git and GitHub section;
- issue-to-agent-to-PR narrative;
- trust and review section.

**Recommended crop**

Retain the full GitHub sidebar and assistant. The empty editor can be cropped
aggressively on narrow layouts.

**Caption**

GitHub workflow and agent task remain in the same workspace.

**Alt text**

ALTAI with its GitHub sidebar open, showing repository controls for pull
requests, issues, branches, commits, and push alongside the Dataset Generator
agent.

---

## 4. `altai-project-management.png`

**Shows**

- Project Management sidebar;
- new task action;
- active, approval, review, and failed status groups;
- open operations;
- agent workspace beside the project board.

**Best placement**

- multi-agent orchestration;
- task governance;
- run operations.

**Recommended crop**

Use the full frame on desktop. For small cards, focus on the left project
sidebar and right agent header.

**Caption**

Task state, approvals, review, and agent work in one operations view.

**Alt text**

ALTAI project-management sidebar with new-task and operations controls,
separate active, approval, review, and failed task states, and the agent
workspace visible on the right.

---

## 5. `altai-slash-command-index.png`

**Shows**

- slash-command index opened from the real composer;
- command names, descriptions, aliases, and categories;
- session commands;
- project-management surface behind the command menu;
- active Dataset Generator persona.

**Best placement**

- commands, skills, and project memory;
- customization section;
- deep feature index.

**Recommended crop**

Crop to the assistant panel and command menu for primary use. A wider version
may retain the project-management sidebar to show system integration.

**Caption**

Built-in and project-defined workflows are searchable from the composer.

**Alt text**

ALTAI assistant with the slash-command menu open, listing session commands such
as new chat, sessions, rename, retry, and stop with their descriptions and
aliases.

---

## 6. `altai-current-workspace.png`

**Shows**

- clean workspace without the assistant;
- file explorer and project root;
- header search;
- status bar and terminal control.

**Best placement**

- supporting “real IDE” comparison;
- before/after interaction;
- documentation, not the hero.

**Caution**

The central area says “No file open.” Do not place a decorative light, glow,
gradient, or hero annotation behind that empty state. The explorer also shows
development-only folders from the audited repository; do not use this capture
uncropped on the public website.

**Caption**

The native workspace before a file or agent task is opened.

**Alt text**

ALTAI desktop workspace showing the project file explorer, global search, and
an empty editor area ready for a file or terminal.

---

## Missing captures worth producing later

These require realistic project data or a prepared demo run:

- editable multi-file diff review;
- plan-diff approval;
- active tool call with concise output;
- notebook with plot artifacts;
- Experiment View with real metrics;
- orchestration DAG with multiple tasks;
- run inspector and evidence panel;
- GitHub issue assigned to an agent;
- MCP server status;
- model picker with cloud and local providers;
- checkpoint/restore surface;
- Afterimage generation run with a quality report.

Do not fabricate these in a design tool. Seed a demo workspace, run the actual
flow, then capture the real UI.
