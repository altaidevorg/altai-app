# Design and Interaction Brief

This is a content and art-direction brief, not implementation code.

## 1. Creative thesis

ALTAI should feel like an instrument panel for serious work, not a marketing
shell wrapped around a chat prompt.

The distinctive image is a **continuous evidence rail**:

> code → run → artifact → metric → review

That rail can recur throughout the site as small, changing labels beside a
code-native replica of the ALTAI application. It explains the product without
turning every section into a feature grid.

## 2. Visual character

### Palette

- Canvas: near-black, not blue-black.
- Primary surface: graphite.
- Raised surface: slightly lighter graphite.
- Primary text: warm white.
- Secondary text: neutral gray.
- Dividers: low-contrast gray.
- ALTAI green: active, selected, running, passed, and primary CTA.
- Amber: waiting/approval.
- Red: failed/blocked/security.
- Blue: informational only when inherited from code or provider identities.

### Non-negotiables

- No gradients.
- No decorative aurora.
- No green border around every card.
- No excessive glass blur.
- No oversized pill collection.
- No heavy neon glow.
- No generic stock illustration.
- No fake terminal text that implies a capability the app does not have.

### Shape

- Small radius on operational cards: 6–10 px.
- Medium radius on large product frames: 14–18 px.
- Buttons may use a capsule shape only for the primary nav/action family.
- Tool output, code, and metric rows should usually be square or minimally
  rounded.

### Typography

Recommended direction:

- Display: a confident neo-grotesk with tight spacing and broad language
  support.
- UI/body: Inter Variable, already present in the product.
- Technical labels: JetBrains Mono, already present in the product.

The hero should be bold without becoming cartoonishly huge. Prefer a
three-line statement with deliberate line breaks over a single 120 px slogan.

## 3. Page rhythm

1. **Hero as product proposition** — short copy plus one interactive app scene.
2. **Category reset** — “The workspace is the agent runtime.”
3. **Continuous loop** — six stages, horizontally or as a scroll-linked rail.
4. **Coding proof** — interactive GitHub, editor, diff, and command states.
5. **ML proof** — execution, artifacts, pilots, notebooks.
6. **Data proof** — Afterimage modes, quality, export.
7. **Orchestration proof** — tasks, dependencies, budgets, review.
8. **Model and extension system** — providers, skills, MCP, commands.
9. **Security and recovery** — boundaries and control.
10. **Final CTA** — direct, minimal, platform-aware.

Alternate dark/light bands are unnecessary. Use spacing, dividers, typography,
and replica scale to create rhythm.

## 4. Signature interactions

### 4.1 Evidence rail

A compact horizontal rail follows one fictional but realistic task:

1. “Map evaluation pipeline”
2. “Pilot 3 quantization paths”
3. “AWQ · 1.8× throughput”
4. “6 files changed”
5. “Tests 42/42”
6. “Ready for review”

Interaction:

- the active stage changes as the section enters the viewport;
- the active product surface changes with it;
- motion uses opacity and a 6–12 px translate, not flying cards;
- reduced-motion users see the final arranged state.

### 4.2 Agent mode switch

Three labels switch one copy/visual frame:

- Coding Agent
- ML Engineer
- Dataset Engineer

The product remains the same interactive workspace; only the active persona,
task starter, and evidence labels change. This is more truthful and more
distinctive than three disconnected feature cards.

### 4.3 Command index reveal

Start with a quiet composer. Typing `/` reveals categories and project
commands. A small side note explains:

> 36 built-ins + any Markdown workflow your repo defines.

Recreate the menu with semantic HTML and the real command data. Typing,
searching, arrow-key navigation, and selection should behave like the product.

### 4.4 Pilot comparison

For the ML section, show three compact experiment rows:

| Candidate | Pilot | Metric | Cost | Result |
|---|---:|---:|---:|---|
| QLoRA | 100 steps | 0.71 | $0.42 | continue |
| DoRA | 100 steps | 0.74 | $0.51 | winner |
| Full FT | preflight | — | $18.30 | reject |

Numbers are illustrative and must be labelled “example run” unless replaced by
real product evidence.

### 4.5 Dataset quality gate

Animate a small set of dataset rows entering a gate:

- grounded;
- diverse;
- schema valid;
- judge agreement;
- exported.

Failed rows should visibly remain failed; never animate every sample into a
success state.

### 4.6 Orchestration map

Use a small DAG:

> Research → Pilot A / Pilot B / Pilot C → Evaluate → Scale → Review

Selecting a node reveals owner, budget, files, environment, and evidence. This
communicates ALTAI’s orchestration model better than a generic “multi-agent”
badge.

## 5. Interactive replica treatment

- Rebuild the visible ALTAI shell in HTML/CSS; do not place PNG captures in the
  public page.
- Reuse the product’s proportions, typography, density, and state colors.
- The replica is a marketing demo, not an iframe and not the production app.
- Use deterministic local state; it must not call models, GitHub, a shell, or
  the visitor’s filesystem.
- Label generated metrics and task results as simulated.
- Preserve readable UI at desktop sizes.
- For mobile, switch to a focused product surface rather than shrinking the
  entire desktop shell.
- Use a neutral one-pixel frame and subtle outer shadow.
- Avoid perspective transforms; they reduce legibility.
- Never add fake green glow behind “No file open.”
- Keep focus visible only while an element is focused.
- Reference PNGs may be used during visual QA, never as rendered page assets.

## 6. Copy hierarchy

Each major section should contain:

1. short monospace eyebrow;
2. outcome-led headline;
3. one paragraph, maximum 55–70 words;
4. one interactive product proof;
5. three to six concrete capability points;
6. optional technical note for expert readers.

Avoid feature walls until the deeper feature index. The landing page should
make the system understandable before making it exhaustive.

## 7. Responsive behavior notes

### Desktop

- The interactive product replica can span 10–12 columns.
- Text blocks should rarely exceed 5–6 columns.
- Sticky evidence rail may remain visible through two related sections.

### Tablet

- Stack copy above the interactive replica.
- Keep agent-mode control horizontally scrollable.
- Convert orchestration DAG into a compact vertical flow.

### Mobile

- Render one focused replica surface at a time.
- Keep the hero to two short copy blocks and one interactive scene.
- Convert long feature lists into disclosure groups.
- Preserve command names, metrics, and status labels at readable sizes.
- No horizontal fake desktop canvas.

## 8. Motion principles

- Motion explains state change, ownership, or progress.
- 160–240 ms for direct interactions.
- 300–500 ms for section-level transitions.
- Avoid infinite decorative animation.
- Use green pulse only for a genuinely running state.
- Stop animations when the tab is not visible.
- Respect `prefers-reduced-motion`.

## 9. Accessibility content notes

- The replica needs a concise accessible summary and live state text.
- Decorative reference captures must not reach the public DOM.
- Do not encode success/failure with color alone.
- Visible focus must not permanently leave a green border after the component
  loses focus.
- Any interactive comparison needs keyboard-operable tabs or buttons.
- Product annotations need a text equivalent.
- Animated metrics must not use rapid count-up effects.
- Body contrast should meet WCAG AA; muted technical labels still need to be
  readable.

## 10. Recommended recurring labels

- LOCAL WORKSPACE
- CURRENT REVISION
- RUNNING
- WAITING FOR APPROVAL
- ARTIFACT READY
- QUALITY GATE PASSED
- READY FOR REVIEW
- MODEL
- BUDGET
- ENVIRONMENT
- EVIDENCE

These labels should look like system state, not promotional badges.
