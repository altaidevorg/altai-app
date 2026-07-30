# Interactive ALTAI App Replica

This specification replaces all public-facing product screenshots with a
code-native, interactive demonstration of ALTAI.

## 1. Purpose

The replica should let a visitor understand ALTAI by using a compact,
deterministic version of the interface:

- switch agent roles;
- inspect a workspace;
- open the terminal;
- type slash commands;
- start a simulated task;
- watch tool and execution state;
- compare ML pilots;
- approve or reject a change;
- inspect a final diff or artifact;
- switch to GitHub and project-management surfaces.

It is not a functional IDE, model client, shell, or GitHub integration. Every
run is local, deterministic demo state.

Persistent disclosure:

> Interactive product tour · simulated run

## 2. App frame

### Window chrome

- ALTAI mark.
- New-workspace button.
- Global search.
- Assistant toggle.
- Settings icon.
- Native-window dots only when presenting the macOS variant.

### Left work surface

Three tabs:

1. Files
2. GitHub
3. Project Management

### Main surface

Possible views:

- code editor;
- unified diff;
- notebook;
- experiment comparison;
- artifact preview;
- empty workspace;
- review summary.

### Bottom drawer

- Terminal tab.
- Split action.
- New terminal action.
- Collapse action.
- Deterministic command log.

### Assistant surface

- task/session tabs;
- active agent;
- permission mode;
- tool activity;
- task starters;
- streamed response;
- todos;
- composer;
- file, snippet, command, diff, and voice controls;
- model picker.

## 3. Visitor controls

Every control must be real HTML, keyboard accessible, and touch friendly.

### Required interactions

- Select Files, GitHub, or Project Management.
- Select Coder, Adaptive ML, or Dataset Generator.
- Select Build or Plan mode.
- Open and close the terminal.
- Focus the composer.
- Type `/` and search the command index.
- Choose a seeded task starter.
- Start, pause, resume, restart, or step through the demo.
- Inspect a tool call.
- Inspect a changed file.
- Toggle unified diff.
- Approve, reject, or restore a simulated edit.
- Open a generated artifact.
- Change from task activity to final review.

### Optional interactions

- Resize the assistant panel within safe min/max bounds.
- Toggle local versus cloud model labels.
- Switch between reduced and full motion.
- Copy a sample prompt.

## 4. Top-level modes

### 4.1 Coding Agent

Seed prompt:

> Add per-user rate limiting to the public API, cover it with tests, and open a
> reviewable change.

Scene sequence:

1. `UNDERSTAND` — maps API routes and the authentication boundary.
2. `PLAN` — lists four implementation steps and two risks.
3. `READ` — opens the relevant API and test files.
4. `EDIT` — updates three files.
5. `TERMINAL` — runs targeted tests.
6. `VERIFY` — 42 tests pass.
7. `REVIEW` — shows a unified diff and security note.
8. `GITHUB` — prepares a pull-request summary.

Visible deterministic tool labels:

- Search workspace
- Read 4 files
- Update 3 files
- Run tests
- Review diff

Final evidence:

- 3 files changed
- 42/42 tests
- 0 security findings
- Ready for review

### 4.2 Adaptive ML

Seed prompt:

> Improve inference throughput for this 7B model on a 24 GB GPU without losing
> more than 1% task accuracy.

Scene sequence:

1. `UNDERSTAND` — target: ≥1.8× throughput and ≤1% accuracy loss.
2. `RESEARCH` — opens three current primary-source notes.
3. `ENUMERATE` — compares AWQ, GPTQ, and FP8.
4. `PILOT` — launches three small jobs.
5. `EVALUATE` — compares speed, memory, accuracy, and cost.
6. `SCALE` — continues the winning pilot.
7. `VERIFY` — checks the original target.
8. `PERSIST` — records the result as workspace knowledge.

Example pilot data, labelled simulated:

| Candidate | Throughput | Accuracy delta | VRAM | Pilot cost | Decision |
|---|---:|---:|---:|---:|---|
| AWQ | 1.92× | -0.6% | 10.8 GB | $0.42 | continue |
| GPTQ | 1.78× | -0.8% | 10.4 GB | $0.39 | reject |
| FP8 | 2.07× | -1.4% | 15.2 GB | $0.55 | reject |

Final evidence:

- Target met
- 1.92× throughput
- 0.6% accuracy loss
- Artifact ready

### 4.3 Dataset Generator

Seed prompt:

> Generate 500 document-grounded support conversations, validate grounding,
> and export training and evaluation splits.

Scene sequence:

1. Inspect 24 source documents.
2. Generate eight personas.
3. Create a 20-dialog pilot.
4. Reject three conversations below the grounding threshold.
5. Auto-improve and re-evaluate.
6. Scale to 500 conversations.
7. Produce train/evaluation split.
8. Export Messages, ShareGPT, and JSONL.

Visible quality metrics, labelled simulated:

- Grounding: 0.94
- Relevance: 0.92
- Diversity: 0.87
- Schema validity: 100%
- Judge agreement: 0.81
- Rejected and regenerated: 17

Final evidence:

- 500 conversations
- 3 export formats
- Dataset card
- Quality report

## 5. Left-surface content

### 5.1 Files

Use a neutral fictional workspace:

```text
aurora-api/
├── .altai/
│   ├── agents/
│   └── commands/
├── docs/
├── notebooks/
├── src/
│   ├── api/
│   ├── models/
│   └── evaluation/
├── tests/
├── ALTAI.md
├── WORKFLOW.md
└── package.json
```

Do not show `.kilo`, local usernames, machine names, private repository names,
or unrelated development folders.

### 5.2 GitHub

Repository:

> altaidevorg/aurora-api

Seed data:

- Pull requests: 1
- Issues: 3
- Branch: `agent/rate-limit`
- Working tree: 3 modified files
- Primary actions: Commit, Push, Open PR

### 5.3 Project Management

Status groups:

- Active
- Approval
- Review
- Failed
- Done

Task graph:

```text
Map API boundary
       ↓
Implement limiter ──→ Add tests
       └─────────────→ Security review
                         ↓
                    Final review
```

Task details:

- agent;
- model;
- environment;
- files;
- budget;
- elapsed time;
- dependencies;
- quality gates;
- evidence.

## 6. Slash-command interaction

Typing `/` opens the actual ALTAI command categories:

- Session
- Workspace
- Code
- Quality
- Project
- Settings

Initial visible commands:

- `/new`
- `/sessions`
- `/compact`
- `/index`
- `/plan`
- `/fix`
- `/test`
- `/review`
- `/paper`
- `/tasks`

One project-defined example:

> `/release-check` · Run the repository’s release-readiness workflow · PROJECT

Keyboard behavior:

- Arrow Up/Down moves selection.
- Enter selects.
- Escape closes.
- Typing filters name, description, alias, category, and source.
- Focus returns to the composer after closing.

## 7. Tool activity content

Collapsed row:

> Read 4 files · 1.2s

Expanded row:

- tool name;
- concise input summary;
- status;
- elapsed time;
- two-to-six lines of output;
- “Open file,” “Open terminal,” or “View artifact” action when relevant.

Do not place rounded containers around every line of tool text. The row itself
can have a divider and state icon; content should remain visually light.

States:

- queued;
- running;
- waiting for approval;
- completed;
- failed;
- cancelled.

## 8. Review interaction

The coding story ends on a real-looking unified diff:

- three files in the file list;
- additions and deletions;
- inline review note;
- test evidence;
- security review result;
- Restore, Reject, and Approve controls.

Selecting “Approve” changes the task to:

> Ready to commit

Selecting “Restore” shows:

> Checkpoint restored · 3 files returned to their previous state

No browser action may alter a real repository.

## 9. Terminal interaction

The terminal is a semantic simulation:

- command lines appear from deterministic scene data;
- visitors can expand logs;
- no arbitrary command execution;
- no hidden input capture;
- no real shell;
- no local username or hostname.

Prompt:

> aurora-api %

Example commands:

```text
pnpm test src/api/rate-limit.test.ts
python pilots/compare_quantization.py --limit 100
afterimage validate -c configs/support-dataset.yaml
```

## 10. Motion timeline

Default autoplay is off.

When Play is selected:

- 0–1.5 s: plan and first tool;
- 1.5–4 s: reads and edits;
- 4–7 s: terminal or execution job;
- 7–10 s: evaluation;
- 10–12 s: review/evidence.

Controls:

- Play/Pause
- Previous step
- Next step
- Restart
- Progress step labels

Animation principles:

- transition opacity and 6–12 px position;
- never animate the whole app window;
- no looping glow;
- no fake typing longer than one short command;
- instant state changes under reduced motion.

## 11. Responsive states

### Desktop, ≥1200 px

- full app shell;
- left surface, main surface, and assistant visible;
- terminal drawer can open without covering the assistant.

### Tablet, 768–1199 px

- left surface collapses to an icon rail;
- main and assistant remain side by side when possible;
- tool details open as a drawer.

### Mobile, <768 px

- replica becomes a focused surface switcher:
  Agent, Workspace, Terminal, Review;
- only one surface is visible at a time;
- progress steps remain available;
- composer and command index remain interactive;
- do not scale the desktop shell down.

## 12. Accessibility

- The replica is contained in a labelled region:
  `aria-label="Interactive ALTAI product tour"`.
- Scene changes announce a concise status through a polite live region.
- Tabs follow the ARIA tabs pattern.
- Menus and listboxes support arrow keys, Enter, and Escape.
- Progress is also available as text.
- Color is never the only status signal.
- Every icon button has a name.
- Focus is visible and clears correctly on blur.
- The run can be paused.
- Reduced motion is fully supported.
- The simulated disclaimer is visible and programmatically associated with the
  run.

Accessible scene summary example:

> Adaptive ML demo, step 5 of 8: evaluating three quantization pilots. AWQ is
> currently the only candidate meeting the accuracy target.

## 13. Accuracy and trust

- Use only capabilities verified in `02-feature-inventory.md`.
- Keep simulated values labelled.
- Do not imply visitor files are being read.
- Do not send composer text to a server.
- Do not request provider keys.
- Do not claim an operation completed outside the demo.
- Do not use competitor names or configuration files inside the replica.
- The replica’s `.altai` configuration belongs to the fictional demo project.

## 14. Analytics events

If analytics are added, record only anonymous interaction events:

- `tour_mode_selected`
- `tour_played`
- `tour_step_changed`
- `tour_command_opened`
- `tour_tool_inspected`
- `tour_review_opened`
- `tour_cta_clicked`

Never record free-form composer input.

## 15. Acceptance checklist

- No PNG screenshot is rendered in the public product tour.
- All three modes complete deterministically.
- The full tour works without network access.
- Keyboard-only use reaches every control.
- Reduced-motion mode remains understandable.
- Mobile uses focused surfaces, not a scaled desktop screenshot.
- No private workspace, username, hostname, token, or repository data appears.
- Every claim maps back to the feature inventory.
- The tour is visibly marked as simulated.
