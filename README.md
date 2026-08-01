<div align="center">
  <img src="public/logo.png" alt="Altai" width="120" />

  # Altai

  **The open agentic development environment.**<br/>
  A local-first, open-source workspace where AI coding agents live inside your editor, terminal, and git — not in a sidebar chat box.

  [![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
  [![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24C8D8.svg)](https://tauri.app)
  [![Platform](https://img.shields.io/badge/platform-macOS%20%C2%B7%20Windows%20%C2%B7%20Linux-lightgrey.svg)](#installation)
  [![Providers](https://img.shields.io/badge/AI%20providers-14-orange.svg)](#bring-your-own-model)
  [![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

  <br/>

  <!-- Drop a demo.gif or screenshot.png into docs/media/ and uncomment: -->
  <!-- <img src="docs/media/demo.gif" alt="Altai demo" width="800" /> -->

  [Features](#features) · [Installation](#installation) · [Multi-Agent Orchestration](#multi-agent-orchestration) · [Providers](#bring-your-own-model) · [Shortcuts](#keyboard-shortcuts) · [Contributing](#contributing)
</div>

---

## Why Altai?

Most AI coding tools are a chat panel bolted onto an editor. **Altai is a full development environment built around agents**: a real PTY terminal the agent can read and drive, a CodeMirror 6 editor with AI autocomplete, native git and GitHub integration, and a multi-agent orchestration engine — all in one fast native window, running on your machine, with your keys, under your control.

- 🔒 **Local-first & private** — API keys live in your OS keychain, GitHub tokens never touch JavaScript, secret files are blocked from agent access by default.
- 🧠 **14 providers, ~60 models** — from Claude and GPT to fully local models on LM Studio and MLX. Your keys, your models, your bill.
- 🤖 **Real multi-agent orchestration** — a task DAG, kanban board, budgets, quality gates, and agent-to-agent mailboxes. Not a demo feature; a 30-module engine.
- 🔬 **Built for ML & research** — reproduce arXiv papers into working code, generate synthetic datasets, run notebooks, and let an Adaptive ML agent guide fine-tuning.
- ♿ **Accessible by design** — full screen-reader support (including the terminal), high-contrast themes, and every shortcut rebindable.

## Features

### Agentic coding, done right

- **Full agent toolset** — `read_file`, `edit`, `multi_edit`, `write_file`, `bash_run`, `bash_background`, `grep`, `glob`, `get_terminal_output`, `todo_write`, `run_subagent`, `suggest_command`, `open_preview`, and more.
- **Four permission modes** — `ask` → `auto-edit` → `plan` → `bypass`, with a deliberate safety lock on full bypass. You choose how much rope the agent gets.
- **Diff-first approvals** — every edit lands as a reviewable diff card; plan mode produces a full plan diff before a single file is touched.
- **Checkpoints & rewind** — pre-edit checkpoints let you restore any file the agent touched, and conversation truncation lets you edit a message and re-run from there.
- **Steering** — redirect a running agent mid-task, or queue your next instruction.
- **Context engineering built in** — automatic compaction with tunable thresholds, tool-result pruning, live context-window and cost meters, and `/compact` when you want it now.
- **Slash commands** — type `/` in the composer to search ALTAI’s command index: sessions, workspace inspection, implementation, quality checks, project workflows, and settings. `/init` writes `ALTAI.md`, `/plan` toggles plan mode, `/paper` imports an arXiv paper, and `/compact` compresses history (`/smol`, `/condense`, and `/summarize` are searchable aliases). Add project-specific workflows as Markdown files in `.altai/commands/`; each file becomes a discoverable `/command-name` and still uses the normal agent approval flow.
- **Composer superpowers** — `@` to attach files, `/` for commands, `#` for reusable snippets, attach your unstaged git diff, terminal output, images, or PDFs. Dictate prompts with voice (Whisper).

### Agents, skills & automation

- **9 built-in agent personas** — Coder, Architect, Code Reviewer, Security, Designer, **Adaptive ML**, **Paper Reproducer**, **Notebook Assistant**, and **Dataset Generator**. Override them, disable them, or write your own with custom instructions and icons.
- **Skills** — install agent skills from any GitHub repo; running agents pick them up without a restart.
- **Automations** — schedule agent runs with `at`, `every`, or cron expressions. Wake up to finished work.
- **Lifecycle hooks** — `session_start`, `before_tool`, `after_edit`, `on_error` and more, defined in `WORKFLOW.md`.
- **Inbox** — notifications, background jobs, and clarification tickets where a blocked agent can ask you a question and resume.

### Multi-agent orchestration

Altai ships a full orchestration runtime, configured per-repo in `WORKFLOW.md`:

- **Task board** — kanban (queued → running → reviewing → done) with quality metrics: first-attempt success, retry rate, verification failures.
- **Task DAG** — dependency graphs with cycle detection and topological scheduling, up to 8 parallel runners.
- **Agent profiles** — per-agent model, reasoning effort, permissions, tools, skills, MCP servers, budgets, and file scopes.
- **Budgets & quality gates** — per-task time/token/cost limits and pass/fail command checks before work can merge.
- **Team coordination** — agent hierarchies, exactly-once mailboxes, and file-conflict detection between agents.
- **Readiness scan** — scores your repo on 9 dimensions for agent-readiness, with evidence links.
- **Run inspector & replay** — durable SQLite ledger, event journal, crash recovery, and full session replay.

### A real IDE, not a chat wrapper

- **Terminal** — true PTY (xterm.js + WebGL), split panes, private tabs, OSC 7/133 shell integration, tab hibernation, WSL support. The agent reads your scrollback and suggests commands straight into it.
- **Editor** — CodeMirror 6 with minimap, Vim mode, split views, breadcrumbs, and **AI inline autocomplete** powered by your choice of ultra-fast model (Cerebras, Groq, local…).
- **LSP** — one-click managed installs for TypeScript, Python, Go, and Rust language servers, checksum-verified.
- **Notebooks** — view, edit, and execute `.ipynb` cells, with an Experiment View for tracking ML runs.
- **Preview** — built-in browser with native webview tabs (yes, Colab works inside Altai).
- **MCP** — per-workspace Model Context Protocol servers with live status and tool probing. Config compatible with Claude Desktop format.

### Git & GitHub, deeply integrated

- Full source-control panel: stage, commit (with AI-generated conventional-commit messages), branch, fetch/pull/push, discard, and **worktrees** — agents can work in isolated worktrees.
- **Git history** with a commit graph rail and per-file diffs.
- **GitHub device-flow OAuth** — the token lives in Rust, never in the webview.
- **Issues, PRs & Projects V2 boards** inside the app — and **Assign Agent**: dispatch an issue or PR to an AI agent with one click, then watch its run live.
- Clone from GitHub, or publish a local project to a new repo.

### Security model

- Secret files (`.env`, `*.pem`, SSH keys, cloud credentials) are blocked from agent reads and writes — symlink-aware.
- Workspace authorization boundary: filesystem and shell commands are confined to authorized roots.
- `.isanagentignore` — gitignore-syntax rules that filter what the agent, explorer, and search can see.
- API keys in the OS keychain; SSRF-hardened local-model proxy with DNS pinning and cloud-metadata blocking.

### Bring your own model

| | Providers |
|---|---|
| **Cloud** | OpenAI · Anthropic · Google · xAI · Cerebras · Groq · DeepSeek · Mistral · Z.AI · Z.AI Coding Plan · OpenRouter |
| **Local** | LM Studio · MLX (Apple Silicon) · any OpenAI-compatible endpoint (Ollama, vLLM, …) |

- ~60 pre-configured model cards with intelligence/speed/cost scores, capability tags (vision, reasoning, tools, coding), context-window limits, and live pricing.
- **Failover model** — if your primary provider rate-limits or goes down, the agent automatically falls back to your chosen backup.
- Per-chat models: run one conversation on Claude Opus and another on a local Llama, side by side.
- Separate, speed-curated model picker for editor autocomplete.

## Installation

Download the latest release for your platform from the [Releases](../../releases) page — macOS (Apple Silicon), Windows, and Linux builds are available.

Altai also integrates with your OS out of the box:

- **Right-click "Explain with AI" / "Refactor with AI" / "Ask About Project"** in Finder and Explorer
- **CLI**: `altai-cli [path]` for the interactive agent, `altai-cli -p "..."` for one-shot runs, and `altai-cli open [path]` for Desktop
- **Deep links** via the `altai://` scheme, Dock menus / Jump Lists, launch-at-login, and single-instance behavior

### Build from source

Prerequisites: [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 20+, and [pnpm](https://pnpm.io).

```bash
git clone https://github.com/altaidevorg/altai-app.git
cd altai-app
pnpm install
pnpm tauri:dev        # development
pnpm tauri:build      # production bundle
```

## Keyboard shortcuts

Every shortcut in Altai is **rebindable** (Settings → Shortcuts), with layout-independent matching that works on international keyboards.

| Shortcut | Action |
|---|---|
| `⌘/Ctrl + I` | Toggle AI panel |
| `⌘/Ctrl + L` | Ask AI about selection |
| `⌘/Ctrl + J` | Toggle terminal drawer |
| `⌘/Ctrl + K` | Keyboard shortcuts dialog |
| `⌘/Ctrl + ⇧ + F` | Find in files |
| `⌘/Ctrl + 1…9` | Jump to tab |

## Project configuration

Altai reads per-repo configuration you can check into version control:

- **`ALTAI.md`** — project instructions injected into every agent run (`/init` writes it for you)
- **`WORKFLOW.md`** — orchestration workflows, lifecycle hooks, and quality gates
- **`.isanagentignore`** — keep files away from agents, search, and the explorer
- **`.altai/agents/`** — agent profiles for orchestration
- **`.isanagent/mcp.json`** — workspace MCP servers

## Tech stack

Tauri 2 (Rust) · React 19 · TypeScript · CodeMirror 6 · xterm.js · Vercel AI SDK · Zustand · Tailwind CSS 4 · Radix UI · SQLite · embedded **IsanAgent** agent runtime (no sidecar process).

## Contributing

Contributions are welcome — bug reports, feature requests, and pull requests alike. If you're changing something substantial, open an issue first so we can align on direction.

Please keep in mind the project's security invariants: agents must never read secret files, API keys never leave the OS keychain, and mutating tools always respect the active permission mode.

## License

[Apache-2.0](LICENSE)

---

<div align="center">
  If Altai is useful to you, consider giving it a ⭐ — it helps others find the project.
</div>
