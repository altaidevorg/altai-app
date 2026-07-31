# ALTAI Multi-Page Product Website

## Codex 5.6 Sol — Implementation Brief

> This document is the source of truth for designing and implementing the
> public ALTAI product website. Read it completely before changing files.

---

## 1. Mission

Design and build a production-quality, multi-page product website for ALTAI.

ALTAI is not a generic AI landing page and not merely a chat panel inside an
editor. It is a local-first, open-source agentic development environment that
works across:

1. A native desktop application.
2. A VS Code extension.
3. A command-line interface.

The product combines three major capability areas:

1. Agentic software development.
2. Agentic project management and multi-agent orchestration.
3. Agentic ML engineering and research workflows.

The website must explain this product clearly, build technical trust, and
convert visitors toward downloading ALTAI, viewing the source on GitHub, or
reading the documentation.

This is not a one-page marketing site. Each major product surface and
differentiator must have a dedicated route with its own narrative, visuals,
metadata, and calls to action.

---

## 2. Working Rules

### 2.1 Start by inspecting the repository

Before implementation:

1. Inspect the existing repository structure.
2. Check for an existing website, design system, framework, hosting
   configuration, and reusable assets.
3. Read the product README, installation guide, and relevant product
   documentation.
4. Locate all available ALTAI logos, icons, screenshots, recordings, and brand
   assets.
5. Identify verified URLs for GitHub, releases, documentation, the VS Code
   extension, and the CLI.
6. Check the worktree for user changes and preserve unrelated work.

Share a concise implementation plan, then proceed without waiting for another
approval.

### 2.2 Keep the website isolated

This task is for the public product website. Do not redesign the ALTAI desktop
application.

If a website project already exists, work inside it and preserve its stack
unless there is a compelling technical blocker.

If no website exists and this brief is being executed inside the desktop
repository, create the website as an isolated application such as:

```text
apps/web/
```

or:

```text
website/
```

Do not mix marketing-site routes or dependencies into the Tauri renderer.

### 2.3 Do not invent product claims or links

Use claims supported by the repository, documentation, source code, or explicit
instructions in this brief.

Never invent:

- Download assets.
- Package names.
- npm packages.
- Homebrew formulas.
- curl installation scripts.
- VS Code Marketplace URLs.
- GitHub repositories.
- Customer quotes.
- Usage metrics.
- Enterprise certifications.
- Pricing.
- Cloud services that do not exist.

Unknown destinations must be represented in central configuration as unresolved
and must not render as broken links.

---

## 3. Verified Product Links

Use these verified public destinations:

```text
GitHub repository:
https://github.com/altaidevorg/altai-app

GitHub organization:
https://github.com/altaidevorg

Latest release:
https://github.com/altaidevorg/altai-app/releases/latest

All releases:
https://github.com/altaidevorg/altai-app/releases

Issues:
https://github.com/altaidevorg/altai-app/issues

License:
https://github.com/altaidevorg/altai-app/blob/main/LICENSE

Installation guide:
https://github.com/altaidevorg/altai-app/blob/main/INSTALL.md
```

At the time this brief was prepared, a verified public VS Code Marketplace URL
and a verified standalone CLI package URL were not available. Search the current
repository and official ALTAI organization before implementation. If they still
cannot be verified, keep them unresolved instead of guessing.

---

## 4. Product Truth

The website should communicate the following product model.

### 4.1 ALTAI is a coding agent

ALTAI can work directly with a repository rather than only answering questions.
Its coding workflows include:

- Reading and searching files.
- Editing and writing files.
- Running foreground and background shell commands.
- Reading terminal output.
- Working with Git and GitHub.
- Producing reviewable diffs.
- Planning before editing.
- Using checkpoints and rewind.
- Receiving steering while running.
- Managing context and compaction.
- Using agents, skills, slash commands, snippets, hooks, MCP servers, and
  automations.

The website should describe outcomes and workflows, not dump internal tool names
into every marketing section.

### 4.2 ALTAI is local-first

Local-first is a product architecture, not a decorative privacy badge.

Communicate that:

- The workspace and execution environment remain under the user's control.
- The native application runs on the user's machine.
- API keys are stored in the operating system keychain.
- Secret files are protected from agent access.
- Filesystem and shell access are restricted to authorized workspace roots.
- Local models are supported.
- Users choose their model providers and control their model costs.
- GitHub authentication is handled outside ordinary webview JavaScript.
- Permission modes, diff review, and checkpoints allow users to control agent
  autonomy.

Do not claim that local-first means offline-only. Users may choose cloud models,
GitHub, Colab, SSH, or remote compute while retaining control of their local
workspace and execution model.

### 4.3 ALTAI is an agentic project-management environment

Project management is a core differentiator, not a minor feature card.

Communicate:

- Local-first task management that does not require GitHub.
- Kanban workflows.
- Task dependency graphs.
- Topological scheduling.
- Parallel agent runners.
- Agent assignment.
- Per-agent profiles.
- Time, token, and cost budgets.
- Quality gates.
- Verification commands.
- Agent hierarchies and mailboxes.
- Worktree isolation.
- File-conflict detection.
- Run inspection.
- Durable event history and replay.
- Crash recovery.
- Repository readiness analysis.
- Optional GitHub Issues, Pull Requests, and Projects integration.

### 4.4 ALTAI is an agentic ML engineer

ML engineering is another primary differentiator and must receive equal product
weight.

Communicate workflows such as:

- Importing and understanding research papers.
- Reproducing papers as working code.
- Generating synthetic datasets.
- Preparing conversational, tool-calling, structured-output, and preference
  datasets.
- Working with notebooks.
- Tracking experiments.
- Designing training and fine-tuning workflows.
- Running evaluation loops.
- Iterating on datasets and model behavior.
- Using local GPUs, Colab, or remote machines over SSH.
- Preserving artifacts, checkpoints, evidence, and reproducibility.

Avoid vague claims like "revolutionize AI." Show concrete ML lifecycle stages.

### 4.5 ALTAI is available through multiple working surfaces

The website should present Desktop, VS Code, and CLI as three interfaces to one
coherent agentic product.

#### Desktop

The full native agentic development environment:

- Editor.
- True PTY terminal.
- Git and GitHub.
- AI agent.
- Project board.
- Orchestration.
- Notebooks.
- Preview/browser surfaces.
- Settings, providers, agents, skills, and MCP.

#### VS Code Extension

The in-editor surface for users who want ALTAI in an existing VS Code workflow:

- Workspace context.
- Agent plans.
- File edits.
- Diff review.
- Tool execution.
- Shared product concepts with Desktop and CLI.

Use the term **VS Code Extension**, not "VSCO plugin."

Do not link to a Marketplace listing unless it is verified.

#### CLI

The terminal-native surface:

- Interactive repository sessions.
- Headless and automation-friendly workflows.
- Scriptable agent operation.
- Project-aware commands.
- Continuity with the broader ALTAI ecosystem.

Only show installation commands that are verified in current product
documentation or release artifacts.

---

## 5. Positioning and Messaging

### 5.1 Primary positioning

Use this as the initial homepage direction:

> **Your local-first agent for code, projects, and models.**

Recommended supporting copy:

> ALTAI works as a native desktop workspace, inside VS Code, and from your
> terminal—combining agentic coding, project orchestration, and ML engineering
> on infrastructure you control.

Primary CTA:

> Download ALTAI

Secondary CTA:

> View on GitHub

### 5.2 Supporting messages

These may be used across appropriate sections:

- One agent. Every place you build.
- Code locally. Orchestrate confidently.
- From issue to implementation. From paper to model.
- Your code, your models, your machine.
- More than a chat panel. An environment built around agents.
- Plan, execute, verify, and review without losing control.
- Run the model you want, where you want.

Do not use all of them on the homepage. Establish one clear message per section.

### 5.3 Voice and tone

The copy should be:

- Precise.
- Technical.
- Confident.
- Calm.
- Concise.
- Evidence-oriented.
- Understandable without oversimplifying.

Avoid:

- Empty superlatives.
- Excessive exclamation marks.
- "Revolutionary" or "game-changing."
- Generic AI-generated prose.
- Fake social proof.
- Long walls of feature bullets.
- Treating developers as beginners.

### 5.4 Audience

Primary:

- Professional software engineers.
- Technical founders.
- AI-native product teams.
- Developers using coding agents.
- ML engineers and applied AI researchers.

Secondary:

- Engineering managers coordinating agent work.
- Open-source maintainers.
- Teams requiring local control over code and credentials.

---

## 6. Brand Assets

Use the real ALTAI logo.

Known source assets in the desktop repository:

```text
public/logo.png
public/icon.png
```

Rules:

- Do not redraw the logo.
- Do not replace it with a generic letter A.
- Do not distort, rotate, recolor, or crop it.
- Preserve its aspect ratio.
- Provide adequate clear space.
- Use the icon with a text-based `ALTAI` wordmark in navigation when
  appropriate.
- Use the logo for favicon/app-icon assets where technically suitable.
- Use the logo or a purpose-built branded composition for Open Graph images.

The navigation identity should generally appear as:

```text
[ALTAI icon] ALTAI
```

The wordmark can use a strong geometric sans-serif treatment, but it must not
compete with the icon.

---

## 7. Visual Direction

The supplied visual reference establishes the desired design language:

- Near-black canvas.
- High-contrast white typography.
- Acid-lime emphasis.
- Thin precision borders.
- Dark technical panels.
- Compact pill navigation.
- Monospace metadata.
- Controlled ambient glow.
- Product interfaces presented as dark framed systems.

Translate this language into a distinct ALTAI website. Do not reproduce the
reference website's exact composition or copy.

### 7.1 Color system

Create semantic tokens rather than scattering raw colors:

```text
canvas
surface
surface-raised
surface-overlay
surface-subtle
foreground
foreground-muted
foreground-faint
border
border-strong
primary
primary-hover
primary-foreground
success
warning
danger
info
focus-ring
```

The primary acid-lime should be used deliberately for:

- Primary calls to action.
- Active navigation.
- Selected product surface.
- Progress.
- Successful or ready states.
- Short highlighted phrases.

Do not color every icon, border, heading, and hover state lime.

### 7.2 Typography

Use:

- A strong, modern sans-serif for display and interface copy.
- A readable sans-serif for body copy.
- A high-quality monospace for commands, paths, metadata, metrics, terminal
  content, and technical labels.

The site should support:

- Large but controlled hero typography.
- Strong section headings.
- Comfortable paragraph width.
- Compact labels.
- Tabular figures for metrics.

Avoid oversized typography that forces every page into the same hero layout.

### 7.3 Surfaces and depth

Use:

- Thin one-pixel borders.
- Small tonal differences between surfaces.
- Minimal shadows.
- Soft inner highlights where helpful.
- Restrained radius.

Marketing cards should feel like precise product surfaces, not floating
glassmorphism tiles.

Use ambient lime glows only in high-impact areas such as:

- Homepage hero.
- Major product transition.
- Download call to action.
- ML workflow visualization.

Keep glow opacity low and ensure it does not reduce text contrast.

### 7.4 Motion

Motion should explain structure:

- Surface switching.
- Workflow progression.
- Terminal activity.
- DAG/task transitions.
- Small reveal transitions.

Rules:

- Keep animations short and subtle.
- Do not animate every scroll section.
- Avoid permanent distracting loops.
- Respect `prefers-reduced-motion`.
- Provide a readable static state when motion is disabled.

---

## 8. Information Architecture

The website must provide real routes. Do not implement the following as anchor
sections on a single page.

Required routes:

```text
/
/product
/desktop
/cli
/project-management
/ml-engineering
/local-first
/download
/open-source
/docs
/changelog
/blog
/about
/privacy
```

If documentation, changelog, or blog content is not yet available, build a
credible route shell with honest empty/upcoming states. Do not generate fake
release notes, articles, or documentation.

---

## 9. Global Navigation

### 9.1 Desktop navigation

Recommended top-level structure:

```text
ALTAI
Product
Project Management
ML Engineering
Local-first
Docs
Download
GitHub
```

`Product` may open a compact dropdown containing:

```text
Overview
Desktop
VS Code Extension
CLI
```

Navigation behavior:

- Sticky or gently floating header.
- Logo and ALTAI wordmark on the left.
- Clear active route state.
- GitHub icon/link remains visible.
- `Download` is the primary navigation CTA.
- External links expose an accessible external-link cue.
- Keyboard navigation and visible focus states are required.

### 9.2 Mobile navigation

Implement a real menu or drawer:

- Logo remains visible.
- Menu is keyboard accessible.
- Escape closes it.
- Focus is managed correctly.
- Background scrolling is controlled.
- All required destinations are available.
- GitHub and Download remain easy to reach.

### 9.3 Footer

Recommended columns:

```text
Product
  Overview
  Desktop
  VS Code
  CLI

Capabilities
  Project Management
  ML Engineering
  Local-first & Security

Resources
  Docs
  Changelog
  Blog
  Download

Open Source
  GitHub
  Releases
  Issues
  License
```

Include:

- ALTAI logo and concise product statement.
- Apache-2.0 reference when appropriate.
- Privacy link.
- Current year.
- No fake social networks.

---

## 10. Page Specifications

### 10.1 Home — `/`

#### Purpose

Explain what ALTAI is within a few seconds, demonstrate its range without
creating confusion, and drive the visitor toward Download, GitHub, Product,
Project Management, or ML Engineering.

#### Recommended sequence

1. Global navigation.
2. Hero.
3. Product proof/showcase.
4. Desktop / VS Code / CLI surface selector.
5. Local-first architecture statement.
6. Coding-agent workflow.
7. Project-management showcase.
8. Agentic ML Engineer showcase.
9. Multi-agent orchestration flow.
10. Model and provider flexibility.
11. Open-source and GitHub proof.
12. Final download CTA.
13. Footer.

#### Hero requirements

Hero content:

- Small local-first/open-source eyebrow.
- Primary headline.
- Short supporting paragraph.
- `Download ALTAI` primary CTA.
- `View on GitHub` secondary CTA.
- Platform availability or open-source metadata.
- One dominant product visual.

Do not place six competing CTAs in the hero.

The hero visual should preferably use:

1. A real desktop screenshot.
2. A real product recording.
3. A polished composition made from real product UI.
4. An honest HTML/CSS product-system visualization.

Do not create fictional features in a fake screenshot.

#### Surface selector

Create an interactive selector for:

- Desktop.
- VS Code.
- CLI.

Each state should show:

- What the surface is best for.
- A distinct visual.
- A short capability summary.
- A link to its dedicated route.

This is a selector, not a carousel that automatically moves while the visitor
is reading.

#### Project-management preview

Show a compact workflow such as:

```text
Backlog → Running → Review → Done
```

Support it with:

- Task dependencies.
- Agent assignment.
- Parallel work.
- Budgets.
- Quality gates.

CTA:

> Explore Project Management

#### ML-engineering preview

Show:

```text
Paper / Data
→ Dataset
→ Experiment
→ Fine-tune
→ Evaluate
```

CTA:

> Explore ML Engineering

#### Local-first section

Use an architectural composition, not a generic shield icon.

Possible relationship:

```text
Your repository
    ↓
Local ALTAI runtime
    ├── Local models
    ├── Your selected cloud provider
    ├── GitHub
    └── Remote compute / SSH
```

The visual should make user control obvious.

### 10.2 Product Overview — `/product`

#### Purpose

Explain the complete ALTAI system and connect its surfaces and capabilities.

#### Content

- Product overview hero.
- "Not a chat wrapper" positioning.
- Shared agent core.
- Agentic coding lifecycle.
- Permission and review model.
- Agents, skills, MCP, hooks, and automations.
- Editor, terminal, Git, notebooks, and preview.
- Desktop / VS Code / CLI comparison.
- Model and provider flexibility.
- Links to project management and ML engineering.
- Download/GitHub CTA.

#### Suggested product architecture visual

```text
                         ALTAI Agent Core
           ┌──────────────────┼──────────────────┐
           ↓                  ↓                  ↓
        Desktop           VS Code              CLI
           │                  │                  │
           └──────── Workspace / Git / Tools ───┘
                              │
               Agents / Skills / MCP / Models
```

This should be responsive and understandable without animation.

### 10.3 Desktop — `/desktop`

#### Purpose

Position the desktop application as the complete native agentic development
environment.

#### Content

- Desktop-focused hero.
- Supported platform callout.
- Product window showcase.
- Editor, terminal, agent, Git, and project workspace.
- Native OS integrations.
- Project management.
- Notebook and preview workflows.
- Security and local execution.
- Platform download cards.
- Link to installation guide.

Use current supported platforms only:

- macOS.
- Windows.
- Linux.

Do not hard-code release asset filenames unless they are resolved from current
release data.

### 10.4 CLI — `/cli`

#### Purpose

Position the CLI as the direct and automation-friendly ALTAI surface.

#### Content

- Terminal-focused hero.
- Interactive repository workflow.
- Headless/automation workflow.
- Project context.
- Agent selection and permissions.
- Scripts, CI, or remote workflows only when verified.
- Relationship to Desktop.
- Verified installation or invocation instructions.
- GitHub/source CTA.

Code examples must be copyable and accurate.

Known desktop-integrated invocations from existing product documentation may
include:

```bash
altai <path>
altai --new-chat
altai --explain
altai --refactor
```

Verify these before publishing. Do not imply a standalone package distribution
if the CLI currently ships only with Desktop.

### 10.5 Project Management — `/project-management`

#### Purpose

Show that ALTAI can coordinate structured agent work, not merely execute a
single prompt.

#### Primary message

Suggested direction:

> Turn agent runs into an engineering system.

#### Content

- Project-management hero.
- Local-first task board.
- Task DAG.
- Parallel execution.
- Agent profiles and assignment.
- Worktree isolation.
- Budgets and quality gates.
- Run inspector.
- Agent coordination and mailboxes.
- Replay and recovery.
- GitHub Issues, Pull Requests, and Projects integration.
- Local workflow without mandatory GitHub connection.
- CTA to Download and relevant Docs.

#### Core workflow visual

```text
Issue or local task
→ Plan
→ Assign agent
→ Create isolated worktree
→ Execute
→ Verify
→ Review
→ Merge or return
```

Use a substantial product-board or DAG visual rather than a grid of twelve small
icons.

### 10.6 ML Engineering — `/ml-engineering`

#### Purpose

Establish ALTAI as an agentic environment for real ML work.

#### Primary message

Suggested direction:

> From research paper to trained model.

Supporting direction:

> Reproduce research, generate datasets, run experiments, and iterate on models
> with an agent that understands the full ML workflow.

#### Content

- ML hero.
- Paper Reproducer.
- Dataset Generator.
- Adaptive ML Agent.
- Notebook Assistant.
- Experiment View.
- Training and fine-tuning.
- Evaluation and iteration.
- Local GPU, Colab, and SSH.
- Reproducibility and artifacts.
- Model/provider flexibility.
- CTA to Download and ML documentation.

#### Workflow visual

```text
Bring data or a paper
→ Generate and validate datasets
→ Design or reproduce experiments
→ Train and fine-tune
→ Evaluate
→ Iterate with evidence
```

Show technical examples such as:

- Dataset rows.
- Experiment runs.
- Evaluation scores.
- Checkpoints.
- Notebook cells.
- Training status.

Do not fabricate benchmark improvements or model-quality numbers.

### 10.7 Local-first & Security — `/local-first`

#### Purpose

Provide a serious trust and architecture page.

#### Content

- Local-first definition.
- Workspace control.
- OS keychain.
- Secret-file protection.
- Workspace authorization boundary.
- Permission modes.
- Diff-first approvals.
- Checkpoints and rewind.
- Local models.
- User-selected cloud models.
- Provider and billing control.
- GitHub token handling.
- MCP boundaries.
- Ignore/config files.
- Open-source auditability.

Avoid generic compliance claims.

### 10.8 Download — `/download`

#### Purpose

Provide one reliable installation destination for every supported surface.

#### Required groups

- macOS.
- Windows.
- Linux.
- VS Code Extension.
- CLI.
- Build from source.

#### Desktop behavior

- Detect the visitor's likely platform only as a recommendation.
- Keep all platforms visible.
- Use GitHub Releases as the source of truth.
- Prefer a stable Latest Release link.
- If resolving assets through the GitHub API, implement a graceful fallback to
  the Releases page.
- Do not make the production build depend on an unauthenticated GitHub API
  request succeeding.

#### Installation trust

Current installation documentation explains that binaries may be unsigned.
Represent this honestly:

- macOS Gatekeeper may warn on first launch.
- Windows SmartScreen may warn on first launch.
- Link to the official installation guide.
- Do not bury the information after the download begins.

#### Build from source

Provide a concise verified path:

```bash
git clone https://github.com/altaidevorg/altai-app.git
cd altai-app
pnpm install
pnpm tauri:dev
```

Verify current prerequisites and commands before publishing.

### 10.9 Open Source — `/open-source`

#### Purpose

Turn open source into a trust and participation story.

#### Content

- Apache-2.0 license.
- Source repository.
- Architecture transparency.
- Build from source.
- Releases.
- Issues and feature requests.
- Contribution path.
- Provider independence.
- Local-first security model.
- Relevant organization projects only when they directly support the ALTAI
  story.

Do not display live GitHub star counts unless the implementation can load them
reliably with a static fallback.

### 10.10 Docs — `/docs`

If a documentation site exists, route or link to it correctly.

If it does not exist, create a structured documentation landing page with
verified categories:

- Getting Started.
- Installation.
- Models and providers.
- Desktop.
- VS Code Extension.
- CLI.
- Agents and skills.
- Project workflows.
- ML workflows.
- Security.
- Build from source.

Do not write fictional detailed documentation solely to fill the page.

### 10.11 Changelog — `/changelog`

Use actual GitHub releases or repository changelog content.

If automated release data is used:

- Cache it.
- Provide a static/error fallback.
- Do not expose API failures as a broken page.
- Do not create release notes that are not present upstream.

### 10.12 Blog — `/blog`

Create the blog index and content model only if needed.

No fake posts, fake authors, fake publish dates, or AI-generated thought
leadership filler.

An honest "Articles are coming" state is acceptable.

### 10.13 About — `/about`

Focus on:

- The problem ALTAI is solving.
- Local-first developer control.
- Open-source direction.
- The relationship between agentic coding, project operations, and ML
  engineering.

Do not invent team biographies.

### 10.14 Privacy — `/privacy`

Use verified product behavior and actual legal copy if available.

If formal legal text is unavailable, provide a clearly labeled product privacy
overview rather than pretending it is a complete legal policy.

---

## 11. Central Link and Site Configuration

Create a single typed configuration source, for example:

```text
src/config/site.ts
```

Suggested model:

```ts
type ExternalDestination = {
  label: string;
  href: string | null;
  status: "verified" | "pending";
};

export const siteConfig = {
  name: "ALTAI",
  description:
    "A local-first agentic development environment for code, projects, and models.",
  links: {
    githubRepository: {
      label: "GitHub",
      href: "https://github.com/altaidevorg/altai-app",
      status: "verified",
    },
    githubOrganization: {
      label: "ALTAI on GitHub",
      href: "https://github.com/altaidevorg",
      status: "verified",
    },
    latestRelease: {
      label: "Latest release",
      href: "https://github.com/altaidevorg/altai-app/releases/latest",
      status: "verified",
    },
    releases: {
      label: "All releases",
      href: "https://github.com/altaidevorg/altai-app/releases",
      status: "verified",
    },
    issues: {
      label: "Issues",
      href: "https://github.com/altaidevorg/altai-app/issues",
      status: "verified",
    },
    cliPackage: {
      label: "CLI package",
      href: null,
      status: "pending",
    },
    documentation: {
      label: "Documentation",
      href: null,
      status: "pending",
    },
  },
} as const;
```

Adapt the shape to the actual project, but retain:

- One source of truth.
- Typed status.
- No fake destinations.
- No duplicated GitHub URLs across page components.

---

## 12. Component Architecture

Build reusable, composable website components.

Recommended primitives:

```text
LogoMark
Wordmark
SiteHeader
ProductMenu
MobileNavigation
SiteFooter
PageHero
SectionHeading
Eyebrow
PrimaryCTA
SecondaryCTA
ExternalLink
ProductWindow
ScreenshotFrame
TerminalDemo
CodeBlock
CopyButton
ProductSurfaceSwitcher
WorkflowSteps
ArchitectureDiagram
FeatureDetail
FeatureGrid
MetricStrip
StatusPill
DownloadCard
PlatformSelector
SecurityCallout
GitHubCTA
EmptyState
```

Guidelines:

- Use semantic HTML.
- Favor composition over page-specific monoliths.
- Avoid one generic card component for every visual problem.
- Avoid duplicating navigation and footer markup across routes.
- Keep page content data separate from deeply nested presentation where useful.
- Use server-rendered/static content by default.
- Add client components only for interactions that need them.

---

## 13. Product Visuals

Use this priority:

1. Real ALTAI screenshots.
2. Real ALTAI recordings.
3. Real repository assets.
4. HTML/CSS diagrams based on real workflows.

Do not:

- Show capabilities that do not exist.
- Generate a fake application screenshot and present it as real.
- Use unrelated stock photography.
- Fill the site with abstract AI brains, robots, or glowing network spheres.
- Use generic laptop mockups when a direct product frame communicates more.

If screenshots are unavailable, build credible technical visuals such as:

- Terminal session.
- Diff approval.
- Task board.
- Agent DAG.
- Training workflow.
- Dataset preview.
- Experiment table.
- Local-first architecture.

Clearly treat conceptual diagrams as diagrams rather than screenshots.

---

## 14. Content Hierarchy

Every major section should answer:

1. What is this?
2. Why does it matter?
3. How does ALTAI do it?
4. What can the visitor do next?

Prefer:

- One strong heading.
- One short explanatory paragraph.
- One meaningful product visual.
- Three to five proof points.
- One contextual CTA.

Avoid:

- Eight equal cards in every section.
- Repeating the same "powerful, fast, secure" claims.
- Long unstructured feature dumps.
- Multiple consecutive sections with identical layouts.

Create rhythm by alternating:

- Full-width product compositions.
- Split text/visual sections.
- Workflow sequences.
- Comparison layouts.
- Technical callouts.
- Focused feature grids.

---

## 15. Responsive Design

Support at least:

- Small mobile.
- Large mobile.
- Tablet.
- Laptop.
- Large desktop.

Requirements:

- No horizontal page overflow.
- Product frames remain legible.
- Wide diagrams have mobile alternatives.
- Tables scroll or transform appropriately.
- CTA groups wrap correctly.
- Navigation remains usable at intermediate widths.
- Typography scales smoothly.
- Touch targets meet accessibility expectations.
- Hover is never required to access information.

Do not merely shrink desktop layouts.

---

## 16. Accessibility

Target WCAG AA.

Required:

- Semantic landmarks.
- Logical heading order.
- Keyboard-accessible navigation and menus.
- Visible focus states.
- Sufficient contrast.
- Descriptive link labels.
- Alternative text for meaningful imagery.
- Decorative images ignored by assistive technology.
- Reduced-motion support.
- Accessible tabs for the product-surface selector.
- Accessible copy buttons and status feedback.
- Dialog/drawer focus management.
- No information communicated only by color.

The lime accent must be tested against every surface on which it is used.

---

## 17. SEO and Sharing

Provide unique metadata for every important route:

- Title.
- Description.
- Canonical URL.
- Open Graph title.
- Open Graph description.
- Open Graph image.
- Twitter card metadata.

Required technical SEO:

- Sitemap.
- `robots.txt`.
- Favicons.
- Web manifest when appropriate.
- Semantic headings.
- Internal links between related product routes.
- Structured data for `SoftwareApplication`.

Potential structured-data fields must be truthful:

- Name.
- Application category.
- Supported operating systems.
- License.
- Source/download URL.

Do not include fake ratings, pricing, or review data.

Suggested search themes:

- local-first coding agent
- open-source coding agent
- desktop AI coding agent
- VS Code coding agent
- CLI coding agent
- multi-agent project management
- agent orchestration for software development
- agentic ML engineer
- AI paper reproduction
- synthetic dataset agent
- local AI development environment

Write naturally; do not keyword-stuff.

---

## 18. Download and Release Integration

GitHub Releases is the source of truth for desktop downloads.

Safe implementation options:

1. Link all platform actions to the stable latest-release URL.
2. Resolve current release assets at build or runtime with caching and a
   reliable fallback.

If using the GitHub API:

- Do not require a private token for basic site rendering.
- Handle rate limits.
- Set a sensible cache or revalidation period.
- Keep the Download page functional when the API is unavailable.
- Never fail the production build solely because GitHub is temporarily
  unavailable.
- Validate asset names before mapping them to platforms.

Platform recommendation:

- User-agent detection may preselect a platform.
- All platform options remain visible.
- Do not automatically download without a click.
- Display architecture distinctions only when actual assets support them.

---

## 19. Technical Baseline

If no website stack exists, use:

- Next.js App Router.
- TypeScript.
- Tailwind CSS.
- React Server Components by default.
- A lightweight component approach.
- Static generation or incremental revalidation where appropriate.

Avoid unnecessary dependencies.

Do not add:

- A CMS unless content requirements demand it.
- A global client-side state library for static marketing content.
- A heavy animation framework for basic transitions.
- A large carousel package.
- A component library that fights the intended design.
- Analytics or tracking without explicit authorization.

If the project already uses another capable stack, preserve it and implement the
same product requirements idiomatically.

---

## 20. Performance

Target a fast production site.

Requirements:

- Optimize images.
- Set explicit image dimensions.
- Avoid layout shift.
- Lazy-load below-the-fold media.
- Keep hero media efficient.
- Subset or optimize fonts.
- Avoid large client bundles.
- Avoid hydrating static sections.
- Use CSS for simple effects.
- Keep animation GPU-friendly.
- Test slow-network and mobile behavior.

Do not sacrifice product clarity for an artificial Lighthouse score, but treat
poor performance as a defect.

---

## 21. Testing and Verification

Run the commands appropriate to the selected website stack.

At minimum verify:

- Type checking.
- Lint.
- Unit/component tests if configured.
- Production build.
- Route rendering.
- Internal links.
- Verified external links.
- Mobile navigation.
- Keyboard navigation.
- Reduced motion.
- Download fallback behavior.
- Missing/pending links.
- No console errors.
- No hydration errors.
- No broken images.

Test the required routes directly, not only by clicking from the homepage.

If browser automation is available, cover these journeys:

1. Home → Download → GitHub Releases.
2. Home → Project Management.
3. Home → ML Engineering.
4. Product → Desktop / VS Code / CLI.
5. Mobile menu → GitHub.
6. Download page platform selection.
7. Keyboard navigation through the header and surface selector.

---

## 22. Implementation Phases

### Phase 1 — Discovery and foundation

- Inspect repository and existing site.
- Inventory real assets.
- Verify product links.
- Establish route structure.
- Create central site configuration.
- Create semantic design tokens.
- Implement logo handling, typography, container, and global layout.

### Phase 2 — Global system

- Header.
- Product menu.
- Mobile navigation.
- Footer.
- CTA system.
- Section heading system.
- Product frames.
- Terminal/code components.
- Workflow and architecture components.

### Phase 3 — Primary conversion pages

- Home.
- Product Overview.
- Download.
- Desktop.

### Phase 4 — Product surfaces

- VS Code Extension.
- CLI.

Handle unresolved distribution links honestly.

### Phase 5 — Differentiator pages

- Project Management.
- ML Engineering.
- Local-first & Security.
- Open Source.

These pages deserve deep content and distinct visuals.

### Phase 6 — Resource pages

- Docs landing.
- Changelog.
- Blog.
- About.
- Privacy.

### Phase 7 — Quality

- Responsive refinement.
- Accessibility review.
- Metadata and structured data.
- Link verification.
- Performance review.
- Production build and automated tests.

---

## 23. Acceptance Criteria

The implementation is complete only when:

### Product and content

- The site clearly identifies ALTAI as a coding agent.
- Local-first is visible in the homepage hero or immediately after it.
- Desktop, VS Code, and CLI are presented as distinct product surfaces.
- Project Management has a dedicated route and substantial content.
- ML Engineering has a dedicated route and substantial content.
- GitHub and open-source positioning are prominent.
- Product claims are supported and concrete.
- No fake testimonials, metrics, customers, or integrations are present.

### Architecture

- The site is multi-page.
- Required primary routes exist.
- Header and footer are shared.
- External destinations are centralized.
- Unknown VS Code/CLI destinations are not invented.
- The website remains isolated from the desktop application.

### Brand and visuals

- The real ALTAI logo is used.
- The visual system follows the black, white, precision-border, and acid-lime
  direction.
- Lime is used deliberately.
- Pages do not all repeat the same hero/card composition.
- Product visuals are real or honestly conceptual.

### Download and GitHub

- Repository link works.
- Latest release link works.
- Releases link works.
- Download page exposes macOS, Windows, and Linux.
- VS Code and CLI availability states are accurate.
- GitHub/API failure does not break the Download page.
- Unsigned-binary guidance is not hidden.

### Quality

- Responsive layouts work on mobile, tablet, and desktop.
- Keyboard navigation works.
- Focus states are visible.
- Reduced motion is respected.
- Metadata is route-specific.
- Production build succeeds.
- Tests and lint pass, or pre-existing unrelated failures are clearly reported.
- No broken routes, images, or verified links remain.

---

## 24. Final Delivery Report

At the end, report:

1. The implemented route list.
2. The main design-system decisions.
3. The reusable components created.
4. The real product assets used.
5. The verified external links used.
6. Any unresolved VS Code, CLI, docs, or release destinations.
7. The download/release strategy.
8. Accessibility and responsive work completed.
9. Test, lint, type-check, and production-build results.
10. Any remaining work that requires product-owner input.

Keep the report concise and factual.

---

## 25. Final Directive

Build a website that makes the product understandable before making it
impressive.

The visitor should leave with five clear ideas:

1. ALTAI is a real coding agent, not a chat wrapper.
2. It works through Desktop, VS Code, and CLI.
3. It is local-first and gives developers control.
4. It can coordinate serious multi-agent project work.
5. It can perform serious ML engineering workflows.

Use the supplied visual reference for art direction, use the real ALTAI logo,
use GitHub as the source of truth for public downloads, and make each major
product capability worthy of its own page.
