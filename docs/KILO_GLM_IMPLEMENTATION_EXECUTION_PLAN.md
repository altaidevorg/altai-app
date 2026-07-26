# Kilo Code + GLM Low-Token Implementation Plan

> Status: proposed companion to `AGENT_OPERATIONS_IMPLEMENTATION_PLAN.md`
>
> Created: 2026-07-26
>
> Goal: implement the ALTAI Agent Operations roadmap through Kilo Code with
> Z.AI GLM models while keeping token use, context repetition, and expensive
> retries under control.

## 1. Scope and correction

This document describes how to build ALTAI from Kilo Code. It does not propose
adding a GLM budget manager, Kilo integration, or model router to the ALTAI
product.

The roles are:

```text
Kilo Code           execution environment
Z.AI / GLM          model provider
ALTAI repository    implementation target
Agent Operations    product roadmap being implemented
```

Kilo Code is officially supported by the Z.AI Coding Plan. Therefore, the
Coding Plan is the preferred route when the developer already has that
subscription. A standard Z.AI API key remains a separate pay-as-you-go BYOK
option.

## 2. Provider setup

### 2.1 Preferred: Z.AI Coding Plan

In Kilo Code:

1. Open model/provider settings.
2. Select `Z AI`.
3. Select the International Coding Plan entrypoint:
   `https://api.z.ai/api/coding/paas/v4/`.
4. Enter a Coding Plan API key.
5. Select an available GLM coding model.
6. Run one small read-only repository task to validate authentication before
   starting implementation.

Use this route when an active Coding Plan subscription already covers the
expected workload.

### 2.2 Alternative: standard Z.AI BYOK

Use generic Z.AI BYOK when pay-as-you-go API billing is preferable or a needed
model is not available through the Coding Plan.

Coding Plan and generic API credentials are separate:

- Do not use a generic Z.AI API key against the Coding Plan endpoint.
- Do not use a Coding Plan key as a generic BYOK key.
- Do not silently fall back from one billing route to the other.
- Confirm the selected provider, endpoint, model, and quota with a small test.

## 3. Model policy

Model availability can differ by plan and Kilo version. Select models from
Kilo's provider model picker and copy the exact model identifier into project
configuration. Do not guess or hardcode an undocumented provider prefix.

Use the cheapest available model that reliably handles each role:

| Work                                          | Preferred tier          | Escalate when                                           |
| --------------------------------------------- | ----------------------- | ------------------------------------------------------- |
| Repository search, file mapping, summaries    | GLM Flash/FlashX or Air | It misses relevant files twice                          |
| Small docs, tests, or isolated UI edits       | GLM-4.7 class           | Tests fail for a non-local reason                       |
| Routine TypeScript or Rust implementation     | GLM-4.7 class           | Design affects persistence or concurrency               |
| Focused diff review                           | GLM-4.7 class           | There is a blocking architectural ambiguity             |
| SQLite migrations, leases, recovery, security | GLM-5 class             | Start with this tier; do not retry upward automatically |
| One hard failure after evidence collection    | GLM-5/5.1 class         | Stop after one high-tier attempt                        |

Rules:

1. Use one default implementation model, preferably the GLM-4.7 class.
2. Reserve GLM-5/5.1 for architecture, concurrency, recovery, and difficult
   debugging.
3. Do not run model bake-offs for the same task.
4. Disable or reduce extended thinking for search, classification, and log
   summarization.
5. Enable deeper thinking only where a wrong design is more expensive than the
   added tokens.
6. If the desired economy model is unavailable in the selected plan, use the
   closest cheaper model shown by Kilo rather than switching endpoints
   mid-task.

## 4. Repository-level Kilo setup

Add the following files in a dedicated setup PR before the first roadmap PR:

```text
kilo.jsonc
.kilo/
  rules/
    altai.md
    token-budget.md
  agents/
    altai-explorer.md
    altai-implementer.md
    altai-reviewer.md
    altai-test-triage.md
```

These files configure the development tool. They must not become ALTAI runtime
dependencies.

### 4.1 Proposed `kilo.jsonc`

The exact provider model IDs must be selected from Kilo before this file is
committed.

```jsonc
{
  "snapshot": true,
  "instructions": [".kilo/rules/altai.md", ".kilo/rules/token-budget.md"],
  "compaction": {
    "auto": true,
    "threshold_percent": 65,
    "prune": true,
    "tail_turns": 2,
    "preserve_recent_tokens": 6000,
    "reserved": 20000,
  },
  "agent": {
    "explore": {
      "model": "<CHEAP_GLM_MODEL_ID_FROM_KILO>",
      "steps": 8,
    },
    "plan": {
      "model": "<DEFAULT_GLM_MODEL_ID_FROM_KILO>",
      "steps": 12,
    },
    "code": {
      "model": "<DEFAULT_GLM_MODEL_ID_FROM_KILO>",
      "steps": 25,
    },
    "debug": {
      "model": "<DEFAULT_GLM_MODEL_ID_FROM_KILO>",
      "steps": 18,
    },
    "review": {
      "model": "<DEFAULT_GLM_MODEL_ID_FROM_KILO>",
      "steps": 12,
    },
    "orchestrator": {
      "model": "<DEFAULT_GLM_MODEL_ID_FROM_KILO>",
      "steps": 10,
    },
  },
}
```

Validate the final keys against the installed Kilo version. Kilo permission
rules use last-match-wins behavior, so broad allow rules must not follow narrow
deny rules.

### 4.2 Project rule: `.kilo/rules/altai.md`

Keep this file short and repository-specific:

```md
# ALTAI repository rules

- Preserve unrelated user changes in the working tree.
- Search with `rg` and `rg --files`.
- Make focused edits; do not perform broad mechanical rewrites without need.
- Do not use destructive Git commands.
- Do not push, merge, open a PR, or modify remote state unless explicitly asked.
- Reuse the existing runtime, event, state, and component patterns.
- Keep GitHub optional; local workflows must continue to work offline.
- For orchestration, treat SQLite and the native coordinator as authoritative.
- Add deterministic tests for state transitions, recovery, and failure paths.
- Read only the named roadmap section and directly related code for each task.
- Report changed files, verification, remaining risks, and the next roadmap item.
```

### 4.3 Token rule: `.kilo/rules/token-budget.md`

```md
# Token budget

- Work on one roadmap item per task.
- Never load the complete roadmap when an exact section is provided.
- Inspect targeted files before expanding the search.
- Do not paste full source files or successful test logs into the conversation.
- Summarize command output; preserve exact text only for failures.
- Keep implementation plans under 500 words unless architecture is requested.
- After two unsuccessful fixes for the same failure, stop and report evidence.
- Use a high-tier model at most once per task unless the user approves more.
- End the task after acceptance criteria pass; start the next item in a new task.
```

## 5. Minimal custom agents

Do not create a different agent for every roadmap item. Four narrowly defined
agents are enough.

### 5.1 `altai-explorer`

- Read-only repository exploration.
- Cheap GLM tier.
- Maximum eight tool rounds.
- Output only relevant files, existing patterns, risks, and unanswered
  questions.
- No implementation plan longer than 300 words.

### 5.2 `altai-implementer`

- Default GLM-4.7-class model.
- Maximum 25 tool rounds.
- May edit only the current roadmap slice and its tests.
- Runs targeted checks before wider checks.
- Stops when acceptance criteria pass or the two-failure rule triggers.

### 5.3 `altai-reviewer`

- Read-only.
- Reviews the current diff and acceptance criteria, not the entire repository.
- Maximum 12 tool rounds.
- Reports only actionable correctness, regression, security, and test gaps.
- Does not rewrite code during review.

### 5.4 `altai-test-triage`

- Cheap/default GLM tier.
- Reads only failing output and directly related code.
- Maximum ten tool rounds.
- Separates product defects, flaky tests, environment problems, and unrelated
  pre-existing failures.

Use Kilo's permission configuration to enforce read-only modes. Validate the
agent front matter and permission syntax with the installed Kilo version before
committing.

## 6. Token envelopes

These are ceilings, not targets. The count includes exploration, planning,
implementation, test diagnosis, compaction, and review.

| Task type                            | Soft warning | Hard stop |
| ------------------------------------ | -----------: | --------: |
| Small docs/config/test task          |          40K |       80K |
| Routine focused implementation       |         100K |      160K |
| Complex Rust/SQLite/concurrency item |         220K |      350K |
| Whole Preview 1 milestone            |         1.4M |      2.0M |

At 70% of a task envelope:

1. Compact context.
2. Stop broad exploration.
3. Keep only the task packet, current diff, failing evidence, and acceptance
   criteria.

At 90%:

1. Do not begin a new fix attempt.
2. Run the cheapest useful verification.
3. Produce a handoff containing completed work, exact failure, and next step.

## 7. Context discipline

The roadmap is intentionally comprehensive and must not be inserted into every
task.

For each Kilo task, provide:

```text
1 roadmap item
1 exact roadmap section
1 bounded file area
3-7 acceptance criteria
relevant constraints
required verification commands
explicit stop condition
```

Do not provide:

- The full 1,000+ line roadmap.
- Earlier completed task transcripts.
- Full successful command logs.
- Multiple alternative designs after a design has been selected.
- Generated summaries that duplicate repository documentation.

Automatic compaction should run around 65% for this project. Preserve the last
two turns, the task packet, unresolved failures, and the current diff summary.
Use a cheap model for compaction if the installed Kilo version supports a
dedicated compaction agent.

## 8. Workflow for every roadmap PR

### Step 1: create a task packet

Write the scope before opening the Kilo task:

```md
Roadmap item:
Roadmap section:
Goal:
In scope:
Out of scope:
Acceptance criteria:
Relevant files:
Required checks:
Stop condition:
Token hard stop:
```

### Step 2: explore

Use `altai-explorer` only when file ownership or existing patterns are unclear.
Skip it for obvious one-file tasks.

Expected output:

- no more than ten relevant paths;
- existing patterns to reuse;
- migration or compatibility risks;
- a short recommended edit sequence.

### Step 3: plan only when needed

Use a separate planning pass for persistence, concurrency, recovery, public API,
or cross-module UI work. Keep the plan below 500 words.

For straightforward edits, the task packet is the plan.

### Step 4: implement one slice

Use `altai-implementer`. Include the exact roadmap subsection, not the complete
document. Keep edits reviewable as one focused PR.

### Step 5: verify outside model reasoning

Let command results determine success. Do not ask the model to predict whether
tests should pass.

Run the narrowest relevant test first, then:

```bash
npm run lint
npm test
npm run build
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Use repository-supported commands if these scripts change. Return only a short
success summary or the exact failing section.

### Step 6: review the diff

Use a fresh, read-only `altai-reviewer` task for risky PRs. Supply:

- task packet;
- changed-file list;
- diff;
- test summary.

Do not supply the implementation transcript.

### Step 7: close context

After the PR is ready:

1. Record acceptance criteria and checks in the PR description.
2. End the Kilo task.
3. Start the next roadmap item with a clean context.

Kilo checkpoints are useful for local recovery, but Git commits and PRs remain
the durable review history.

## 9. Subtask policy

Kilo's new-task flow creates a separate history and pauses the parent task. Use
it only when context isolation saves more tokens than the handoff costs.

Good uses:

- one read-only architecture investigation;
- one isolated implementation slice;
- one fresh diff review.

Avoid:

- planner, implementer, debugger, and reviewer subtasks for a tiny edit;
- nested orchestration;
- splitting tightly coupled database and coordinator changes between parallel
  tasks;
- multiple agents editing the same files;
- leaving the parent task open across several PRs.

Default to one active implementation task. Use at most two independent tasks
only when their file ownership and acceptance criteria do not overlap.

## 10. Prompt template

Use this template for implementation:

```md
Implement roadmap item <ID> from
docs/AGENT_OPERATIONS_IMPLEMENTATION_PLAN.md, section "<SECTION>".

Goal:
<ONE SENTENCE>

Read first:

- <PLAN SECTION>
- <UP TO FIVE FILES OR DIRECTORIES>

In scope:

- <BOUNDED CHANGE>

Out of scope:

- Later roadmap items
- Unrelated cleanup
- Remote Git/GitHub actions

Acceptance criteria:

1. <OBSERVABLE RESULT>
2. <OBSERVABLE RESULT>
3. <TEST OR INVARIANT>

Constraints:

- Preserve unrelated working-tree changes.
- Reuse existing ALTAI patterns.
- Use targeted search and do not load the whole roadmap.
- Stop after two unsuccessful attempts at the same failure.

Verify:

- <TARGETED COMMAND>
- npm run lint
- npm test
- npm run build

Hard token stop:
<BUDGET>

Finish with:

- changed files;
- checks run;
- remaining risks;
- no work on the next roadmap item.
```

## 11. First implementation sequence

Execute the first six roadmap PRs in separate Kilo tasks:

| Order | Roadmap item                           | Model tier                            | Hard stop | Reason                                                 |
| ----: | -------------------------------------- | ------------------------------------- | --------: | ------------------------------------------------------ |
|     1 | O1 — domain state transitions          | GLM-4.7; GLM-5 only for design review |      250K | Establishes invariants for everything else             |
|     2 | O2 — SQLite ledger                     | GLM-5                                 |      350K | Schema, migrations, idempotency, and crash safety      |
|     3 | O3 — mock runner and coordinator tests | GLM-4.7                               |      200K | Deterministic behavior with limited architectural risk |
|     4 | O4 — Rust coordinator and Tauri events | GLM-5                                 |      350K | Async ownership and event ordering                     |
|     5 | O5 — legacy recovery                   | GLM-5                                 |      300K | Compatibility and restart correctness                  |
|     6 | O6 — native runner v2                  | GLM-4.7; escalate once                |      250K | Bounded adapter implementation                         |

First milestone ceiling: 1.7M tokens. Preview 1 absolute ceiling: 2.0M tokens.
If this milestone exceeds 1.7M, stop before widening scope and review which
contexts, logs, or retries caused the overrun.

### 11.1 O1 task packet

```md
Implement O1, "Domain state transitions."

Create the smallest authoritative domain model for task, attempt, lease, and
event transitions. Make invalid transitions impossible or explicitly rejected.
Do not add SQLite or UI work.

Acceptance:

- Transition rules are centralized.
- Invalid transitions return typed errors.
- Unit tests cover every allowed terminal transition and representative
  invalid transitions.
- Existing local project-board behavior remains unchanged.

Hard stop: 250K.
```

### 11.2 O2 task packet

```md
Implement O2, "SQLite ledger."

Persist the O1 domain model with migrations, idempotent writes, and restart-safe
queries. Do not start the coordinator.

Acceptance:

- Schema creation and migration are deterministic.
- Events are append-only and attempts retain history.
- Duplicate idempotency keys do not create duplicate work.
- Restart and migration tests pass.

Hard stop: 350K.
```

### 11.3 O3 task packet

```md
Implement O3, "Mock runner and coordinator tests."

Define the runner boundary and a deterministic fake. Test claim, start, event,
completion, cancellation, retry, and lost-lease behavior without launching a
real model.

Acceptance:

- Tests use a deterministic clock and runner.
- Success, failure, cancellation, retry, and recovery are covered.
- No network or model provider is required.

Hard stop: 200K.
```

### 11.4 O4 task packet

```md
Implement O4, "Rust coordinator and Tauri events."

Add the native coordinator loop on top of O1-O3. Keep SQLite authoritative and
make renderer events projections of committed state.

Acceptance:

- A single owner claims eligible work.
- Lease renewal and shutdown are deterministic.
- Events are emitted only after committed state changes.
- Restart and duplicate-event behavior is tested.

Hard stop: 350K.
```

### 11.5 O5 task packet

```md
Implement O5, "Legacy recovery."

Recover legacy/in-flight work into the new ledger without silently duplicating
or discarding runs.

Acceptance:

- Migration is idempotent.
- Recoverable runs resume or become explicitly retryable.
- Unrecoverable state is visible with a typed reason.
- Repeated startup produces the same result.

Hard stop: 300K.
```

### 11.6 O6 task packet

```md
Implement O6, "Native runner v2."

Implement the next native runner adapter against the O3 boundary. Preserve
streaming events, cancellation, exit status, and bounded output capture.

Acceptance:

- The adapter passes the shared runner contract.
- Cancellation terminates child work and records the outcome.
- Output limits and failure mapping are tested.
- No provider-specific state leaks into the coordinator.

Hard stop: 250K.
```

## 12. Quality gates

Every PR must satisfy:

- scope matches one roadmap item;
- no unrelated file changes;
- changed behavior has deterministic tests;
- TypeScript and Rust formatting pass where relevant;
- lint, tests, and production build pass;
- persistence changes include migration and restart coverage;
- concurrency changes include cancellation and recovery coverage;
- reviewer reports no unresolved blocking finding;
- PR description records commands and known risks.

If a repository-wide check fails for a pre-existing unrelated reason, record
the exact failure and prove the targeted check passes. Do not spend the task
budget repairing unrelated failures.

## 13. Cost review

Track cost outside ALTAI using Kilo's displayed usage and the selected Z.AI
plan/quota.

At the end of each roadmap PR, record:

```text
model(s)
approximate input/output tokens
number of compactions
number of failed fix attempts
whether high-tier escalation was used
```

After the first three PRs, calibrate future envelopes using actual usage.
Reduce context first; do not immediately downgrade implementation quality.

## 14. Definition of ready

Kilo-driven implementation can start when:

- provider authentication succeeds with a read-only test;
- exact GLM model IDs are selected in Kilo;
- `kilo.jsonc` is validated by the installed Kilo version;
- project rules and four custom agents are installed;
- the O1 task packet is reviewed;
- the working tree has a known baseline;
- verification commands run once before roadmap changes.

## 15. References

- Z.AI Kilo Code setup:
  <https://docs.z.ai/devpack/tool/kilo>
- Z.AI Coding Plan quick start:
  <https://docs.z.ai/devpack/quick-start>
- Z.AI pricing:
  <https://docs.z.ai/guides/overview/pricing>
- Z.AI context caching:
  <https://docs.z.ai/guides/capabilities/cache>
- Z.AI thinking mode:
  <https://docs.z.ai/guides/capabilities/thinking-mode>
- Kilo BYOK authentication:
  <https://kilo.ai/docs/gateway/authentication>
- Kilo custom modes:
  <https://kilo.ai/docs/customize/custom-modes>
- Kilo custom rules:
  <https://kilo.ai/docs/customize/custom-rules>
- Kilo context condensing:
  <https://kilo.ai/docs/customize/context/context-condensing>
- Kilo checkpoints:
  <https://kilo.ai/docs/code-with-ai/features/checkpoints>
- Kilo new-task tool:
  <https://kilo.ai/docs/automate/tools/new-task>
