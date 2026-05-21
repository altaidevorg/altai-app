# altai Web UI — Positioning & Plan

> Status: planning. No code yet. Companion to [REMOTE_ROADMAP.md](./REMOTE_ROADMAP.md) — depends on Phase B1 (cloud worker) landing first or in parallel.

## One-sentence positioning

> **altai web is "Linear for agents" — a project-management-grade home for
> orchestrating long-running agent sessions across machines, repos, and providers.
> Not an IDE in a tab.**

The desktop app is where you *work alongside* an agent. The mobile app is where
you *check in*. The web app is where you *run a portfolio of agents* — assign,
compare, queue, review, sign off.

## What the web UI is NOT

We borrow from the market survey to set hard anti-scope. These are deliberate
non-goals, not future-work placeholders.

- **Not a browser IDE.** No file tree + editor + terminal in a tab. bolt.new,
  Replit, Devin and GitHub Spark already occupy that space and run a Node
  environment via WebContainers or cloud VMs. We can't out-execute them and
  shouldn't try.
- **Not a prompt-to-app generator.** v0, Lovable, bolt and Spark target
  founders/designers shipping a starter app from a prompt. Our user already
  has a repo.
- **Not a clone of the desktop.** Anything that looks like Cursor-in-a-browser
  loses to Cursor. The desktop and the web have different shapes for different
  moments.
- **Not artifact-as-app.** Claude Artifacts and ChatGPT Canvas do the
  "the chat output is the deliverable" thing well. We are not a one-off-tool
  generator.

## Where it sits in the constellation

```
                       ┌──────────────────────────┐
                       │     altai web (this)     │
                       │  - sessions board        │
                       │  - bake-off + review     │
                       │  - team / project mgmt   │
                       └────────────┬─────────────┘
                                    │  WS / REST
                                    ↓
   ┌──────────────────┐   ┌──────────────────────┐   ┌──────────────────┐
   │  Desktop altai   │ ←→│  altai cloud worker  │←→ │  Mobile altai    │
   │  (Tauri)         │   │  (Phase B1)          │   │  (PWA → native)  │
   │  drives work     │   │  state, agent loop,  │   │  review on phone │
   └──────────────────┘   │  push fan-out        │   └──────────────────┘
                          └──────────────────────┘
```

Each surface has one job. Cross-surface contamination ("the web has a terminal
because it's cool") is what makes products mushy; resist it.

---

## Market read — what the survey told us

Detailed findings: see the research notes alongside this doc (research summary
in the conversation that produced this plan, products covered are listed in the
sources block below). Three patterns that matter for *our* positioning:

**Recurring patterns we will adopt selectively.**
- Plan-then-execute gate (Devin, Replit, Cursor). We already have plan mode; the
  web should be where you approve plans for sessions you launched from your phone.
- Kanban of in-flight agents (Cursor web, Devin parallel runs, Continue Mission
  Control). Closest competitor surface — but every shipped version is single-user
  + shallow. Room to go deeper.
- Plan-mode-first / cost-tier picker (Replit Lite/Economy/Power). Per-session
  budget caps make agents cheap to delegate. We should expose this.
- Voice as input. Universally absent. Open opportunity.

**Patterns we will not adopt.**
- One-click deploy CTA. We are not shipping the user's app for them.
- Embedded WebContainer or cloud VM IDE. See anti-scope above.
- "Visual edit on canvas" pixel-pushing. Out of scope; that's a UI-builder play.

**Three differentiation angles we can credibly own** (vs. the strongest current
competitor in each):
1. **Multi-session orchestration with structure** — beat Cursor's Kanban by
   adding dependencies, owners, sprints, SLAs (real Linear-grade primitives).
2. **Cross-agent bake-off UI** — Cursor advertises "run in parallel"; nobody
   ships a real side-by-side diff-of-diffs with auto-eval. Open lane.
3. **Local-repo + cloud-agent hybrid review** — the web view of a session whose
   executor is the user's own desktop machine. This is the local↔cloud gap
   nobody closed.

(Voice-first and team spectating are also live opportunities; they're in the
phasing table below as later swings.)

---

## Core surfaces

Five top-level views. Each one earns its place.

### 1. Sessions board (the home)

The default landing surface. Kanban or list-toggle, with columns:
`Queued → Running → Awaiting approval → Done → Failed`.

Each card carries:
- title, repo, executor (local/SSH/docker/cloud), model, current step
- live progress dots (tools called, tokens, wall time, $ spent)
- owner avatar; tag chips; SLA / deadline if set

Drill into a card → **Session timeline view** (see #2).

Differentiator: Cursor web stops at this surface. We layer the rest on top.

### 2. Session timeline

The runtime view of a single session.

Left column: scrollable timeline of messages, tool calls, file edits with
inline diff. Right column: contextual pane that swaps based on focus — file
diff, terminal output snapshot, plan tree, env metadata.

Critical interactions:
- **Approve / deny** pending tool call (mirrors mobile flow).
- **Branch** at any point: "what if we tried this differently from here?" forks
  a sibling session.
- **Comment** at a step: attach a note to a specific event. Surfaces in the
  desktop client too.
- **Pause / resume / abort.**

This is the surface where *team spectating* could live in a later phase
(presence cursors, shared comments).

### 3. Bake-off

Open the same task against N agents/models. Each runs in its own column.

What's shown side by side: final diff per agent, total tokens & dollars, wall
time, automated rubric scores (lint pass, tests pass, type-check, optional LLM
judge). Pick a winner; the patch becomes the canonical output, the losing
sessions get archived with full trace for postmortem.

Two flavors at launch:
- **Manual bake-off** — user picks the agents.
- **Suggested bake-off** — for a given task class (e.g. "tricky frontend
  refactor"), the system pre-fills the recommended lineup.

This is the unique surface. None of the surveyed products ship it credibly.

### 4. Projects

Linear-grade primitives wrapping sessions:
- Project = a repo + default executor + default agent + budget caps.
- Tasks = a queue of sessions to run (with optional dependencies between them).
- Owners, labels, due dates, watchers.
- "Run all queued" / "Run when CI green" automation hooks.

This is what separates "agent dashboard" from "agent program management." It
also gives teams a reason to invite collaborators.

### 5. Settings / org

Standard: keys, billing, executors registry (SSH hosts, container endpoints),
agent registry (file-based agents shared at the org level), webhooks, audit log.

Audit log matters more than usual: every tool execution by an autonomous agent
is an org-level event. Shipping this from day one is cheap insurance.

---

## What's deferred (named, not built)

These appear in the survey but are not in launch scope:
- **Voice input.** Compelling but premium-niche; add in v0.2 once core surfaces
  ship. Web Speech API as the cheap first cut.
- **Generated wiki / codebase Q&A** (Devin Wiki / Cody style). Real value, but
  it competes with Sourcegraph head-on. Defer to v0.3+ once we know the
  agent-graph is the differentiator.
- **Slack / Linear ingress.** Triggering sessions from outside the app is a
  growth lever, not a launch feature. Add after sessions board ships.
- **Multi-user real-time co-spectating.** Strong opportunity but expensive.
  Phase v0.4.
- **PWA / install-as-app.** Cheap to add late; not a v0.1 priority.

---

## Phasing

Each row is shippable on its own. Don't bundle.

| v | Scope | Sessions est. | Depends on |
|---|---|---:|---|
| **0.1** | Sessions board (#1) + Session timeline read-only (#2) | 3-4 | B1 cloud worker (read-only WS subscription) |
| **0.2** | Tool approval + comments + pause/abort in timeline | 2 | v0.1, mobile approval protocol agreed |
| **0.3** | Projects + tasks + executor registry (#4, #5) | 3-4 | v0.1 |
| **0.4** | Bake-off (#3), manual mode only | 3 | v0.2 (need per-session pause + diff capture) |
| **0.5** | Bake-off auto-rubric (lint/tests/typecheck/LLM judge) | 2 | v0.4 |
| **0.6** | Local-repo executor bridge — web session whose executor is the user's desktop machine over a tunnel | 3 | desktop tunnel API |
| **0.7** | Multi-user spectating (presence, shared comments) | 3 | v0.2 |
| **0.8** | Voice input on timeline ("dictate next instruction") | 1-2 | Web Speech API |

Total to v0.5 ("the differentiated product"): ~14-17 sessions of work, *after*
the cloud worker is real.

Hard rule: don't start v0.x until B1 of the remote roadmap has a passing WS
contract and at least one running session that the web can attach to. Building
the web against mocks is fine for v0.1 layout, but the *real* test is end-to-end
against the worker.

---

## Tech stack — opinionated

Pick one row, don't shop.

| Layer | Choice | Why |
|---|---|---|
| Framework | **Next.js 16 (App Router) + Turbopack** | RSC for the dashboard (cheap, cacheable), client components only for the timeline and bake-off views. Already in the cutoff window; we own no native code. |
| UI | **shadcn/ui + Tailwind v4** | Matches desktop look-and-feel; the desktop app already pulls shadcn (`components.json` present). Reuse component primitives. |
| State | **Zustand on the client; URL state for views** | Same store library as desktop. Don't reach for Redux/jotai. |
| Realtime | **WebSocket from the cloud worker; SSE for read-only public views** | WS for authenticated sessions; SSE for shared/spectator links — they're cacheable and simpler. |
| Auth | **JWT issued by the cloud worker; OAuth (GitHub) for sign-in** | Worker is the source of truth. Web just trades GitHub OAuth for a worker JWT. |
| Hosting | **Vercel** | Aligns with Next; one less platform to manage. The cloud worker lives elsewhere (Fly.io / Railway / Render — decided in B1). |
| Tests | **Playwright** for the journey; Vitest for components | Already what the desktop uses for unit tests. |

Two things to *not* pull in:
- A drag-and-drop board library (`@dnd-kit` and friends) until v0.3. v0.1's
  board is a list with status filters; drag adds 80% of the bugs and 5% of the
  value at launch.
- A heavyweight diff component (Monaco). The desktop has CodeMirror; the web
  can render diffs with a tiny prism-grammar approach in v0.1. Upgrade only
  if v0.4 bake-off needs side-by-side editing.

---

## Branding for the web

The desktop is "altai." The web should be `altai.dev` or `altai.app` (or a
subdomain like `cloud.altai.dev` if those are taken). The web product name in
copy stays just "altai" — we are not branding it "altai Cloud" or "altai
Sessions." Surface name discipline saves a year of internal confusion.

---

## Open questions to lock before code

1. **Single-tenant or multi-tenant from day 1?** Multi-tenant tax is real; if
   we're not selling team seats in 12 months, ship single-tenant first and add
   tenancy when the second customer signs.
2. **Pricing model.** Bring-your-own-key vs. metered access. Affects the
   bake-off UX because metered usage punishes parallel runs. Decide before v0.4.
3. **Where does the timeline data live long-term?** Worker SQLite is fine for
   weeks; we'll want Postgres + S3 for transcripts past 30 days. Plan the
   migration before v0.3.
4. **Public share links?** "Anyone with the link can view this session" is one
   of the loudest patterns in the survey (Claude, bolt, v0). Decide v0.1 or
   v0.3 — earlier is louder, later is safer.
5. **Do we surface "executors" to end users as a concept?** The survey shows
   most products hide the execution substrate. We should hide it for v0.1 (one
   default executor per project) and expose it in v0.3 when projects ship.

---

## Sources

The product survey backing this plan covered: v0.app, bolt.new (StackBlitz),
Lovable, Replit Agent, Cursor (web), Devin (Cognition), Claude.ai Artifacts,
ChatGPT Canvas, Cody for Web (Sourcegraph), Windsurf, aider --browser,
Continue Hub / Mission Control, E2B, Open Interpreter, GitHub Spark, GitHub
Copilot Coding Agent, Perplexity Labs, Manus, 21st.dev Magic, Codebuff,
Magic.dev (no shipped product), Phind (shutting down Jan 2026). Findings
synthesized from each product's official site, changelog, and recent (last 12
months) coverage.

---

## What to do next

If you agree with the positioning:

1. **Lock the answers** to the five open questions above. None require code.
2. **Wait on B1.** v0.1 cannot start before the cloud worker has a real WS
   contract — building against mocks for more than one phase compounds rework.
3. **Mock the sessions board in Figma or a static Next.js page** to test the
   information architecture. Cheap, catches IA mistakes before code.

If positioning needs to shift (e.g. you want artifact-as-app or browser IDE
after all), revisit *before* phasing — that decision invalidates phases 0.1-0.5.
