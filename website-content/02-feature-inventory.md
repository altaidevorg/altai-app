# Source-Verified Feature Inventory

Snapshot date: 2026-07-29

This inventory is intentionally broader than the landing-page copy. It is the
source pool for product pages, documentation, comparison tables, launch posts,
and future feature-specific campaigns.

## Status key

- **ALTAI shipped** — present in the current `altai-app` source or README.
- **IsanAgent embedded** — present in the current IsanAgent `main` consumed by
  ALTAI.
- **Afterimage available** — present in the current Afterimage `main` and
  available to ALTAI’s ML/data agents.
- **Experimental** — usable but should be described carefully.
- **Roadmap** — documented direction; do not market as shipped.

---

# 1. ALTAI application

## 1.1 Native workspace

Status: **ALTAI shipped**

- Native desktop application built with Tauri 2, Rust, React, and TypeScript.
- macOS, Windows, and Linux distribution.
- Local-first workspace model.
- Single-instance behavior and persisted window state.
- Launch-at-login support.
- Native update flow.
- `altai://` deep links.
- CLI entrypoints for opening a path, starting a new chat, explaining code, and
  refactoring.
- Finder and Explorer context actions.
- Dock menu and Windows Jump List integration.
- Workspace welcome and workspace authorization gate.
- Environment selector in the status bar.

## 1.2 File explorer and workspace navigation

Status: **ALTAI shipped**

- Native file tree.
- Expand/collapse folder navigation.
- File and folder creation.
- Rename, delete, duplicate, reveal, and open-with actions.
- File and folder icon resolution.
- Git decoration state in the explorer.
- Search inside the explorer.
- Workspace-wide search.
- Grep and glob-backed discovery.
- File watching and change propagation.
- Hidden-file visibility controls.
- `.isanagentignore` enforcement in the explorer and agent-facing searches.
- Symlink-aware path handling.

## 1.3 Editor

Status: **ALTAI shipped**

- CodeMirror 6 editing surface.
- Syntax support for JavaScript/TypeScript, JSON, Markdown, HTML, CSS, Python,
  Go, Rust, PHP, and additional legacy modes.
- Multiple editor groups and split views.
- Breadcrumb navigation.
- Minimap with markers.
- Search and replace.
- Language-aware lint integration.
- Code folding and bracket behavior.
- Vim mode.
- Run-file support.
- Markdown preview.
- Image preview.
- Git diff panes.
- AI diff panes.
- Split diff view.
- Diff cache and selectable diff-view mode.
- Editor tab stack and per-tab state.
- AI inline autocomplete.
- Dedicated autocomplete model selection.

## 1.4 Language intelligence

Status: **ALTAI shipped**

- LSP client and JSON-RPC transport.
- Managed language-server catalogue.
- Managed installs for TypeScript, Python, Go, and Rust.
- Checksum-verified downloads.
- Node runtime discovery and installation support.
- Install progress and lifecycle handling.

## 1.5 Terminal

Status: **ALTAI shipped**

- Real PTY backend.
- xterm.js terminal with WebGL rendering.
- Split terminal panes.
- Terminal tabs and pane tree.
- Private terminal tabs.
- Shell selection and session lifecycle.
- OSC 7 working-directory integration.
- OSC 133 command-boundary integration.
- Terminal output available as agent context.
- Agent command suggestions inserted into the terminal.
- Search, fit, serialize, and web-link add-ons.
- Dormant terminal ring and tab hibernation.
- Windows and WSL-aware handling.
- Data-availability filtering and terminal safety checks.
- Rebindable terminal shortcuts.

## 1.6 Notebooks and experiments

Status: **ALTAI shipped**

- `.ipynb` parsing and editing.
- Cell-by-cell notebook rendering.
- Code and Markdown cells.
- Rich MIME output rendering.
- Cell execution.
- Notebook state management.
- Experiment-oriented view for ML runs.
- Notebook Assistant built-in agent.

## 1.7 Preview and webview

Status: **ALTAI shipped**

- Built-in preview panes.
- Native webview tabs.
- Address bar and navigation.
- Local-development preview support.
- Embedded external tools, including Colab-compatible workflows.
- Preview stack alongside code and agent surfaces.

## 1.8 Git

Status: **ALTAI shipped**

- Repository status.
- Stage and unstage.
- Commit.
- AI-assisted conventional commit message generation.
- Fetch, pull, push, and publish.
- Branch list, create, switch, and delete.
- Remote status.
- File-level diffs.
- Discard changes.
- Stash-related operations.
- Worktree creation and management.
- Git history pane.
- Commit graph rail.
- Per-commit and per-file diff inspection.
- Safe process layer and parsed Git errors.

## 1.9 GitHub

Status: **ALTAI shipped**

- GitHub device-flow OAuth.
- Native token handling in Rust.
- Clone from GitHub.
- Publish a local project to GitHub.
- Issues list and detail.
- Pull-request list and detail.
- Create issues and pull requests.
- Project V2 board integration.
- Linked GitHub project selection.
- Local and GitHub work items in a common management surface.
- Assign an issue or pull request to an agent.
- Configure the assigned agent, model, permissions, and execution options.
- Track assignment runs.
- Review and publish agent work.
- Pull requests, issues, branch, commit, and push controls in the sidebar.

## 1.10 Accessibility and input

Status: **ALTAI shipped, active audit**

- Rebindable keyboard shortcuts.
- Layout-independent shortcut matching for international keyboards.
- Global shortcuts dialog.
- Live-region announcements.
- Screen-reader event bridge.
- Terminal accessibility mode.
- Visible focus treatment.
- High-contrast and theme support.
- Reduced-motion behavior.
- Accessibility audit documentation across chat, editor, terminal, diff,
  notebook, preview, and deep panes.

---

# 2. Coding-agent experience

## 2.1 Built-in coding tools

Status: **ALTAI shipped / IsanAgent embedded**

Core tools represented in the current product and runtime include:

- read file;
- write file;
- targeted edit;
- multi-edit;
- directory listing;
- glob;
- grep;
- workspace search;
- shell command run;
- background command execution;
- terminal-output read;
- task/todo writing;
- subagent launch;
- command suggestion;
- preview opening;
- context compaction;
- tool-result recall;
- execution-session management;
- execution job list/status/result/cancel;
- artifact list;
- environment information;
- execution log read;
- cron/scheduling;
- skill discovery and instruction loading;
- workspace ignore management;
- clarification requests;
- memory search and persistence;
- web and paper research helpers;
- ML-domain and workflow tools.

## 2.2 Permission and approval model

Status: **ALTAI shipped**

- Ask mode.
- Auto-edit mode.
- Plan mode.
- Bypass mode with explicit safety treatment.
- Tool-level approval UI.
- Edit approval cards.
- Diff-first review.
- Plan diff review before implementation.
- Permission-mode switcher in the composer.
- Mutating-tool policy enforcement.
- Workspace-root authorization.
- Secret-path denial.
- Command suggestion instead of silent terminal injection.

## 2.3 Recovery and steering

Status: **ALTAI shipped / IsanAgent embedded**

- Pre-edit checkpoints.
- One-step restore for touched files.
- Conversation rewind by truncating after a chosen user message.
- Retry/regenerate last turn.
- Stop/cancel active run.
- Mid-run steering.
- Queued follow-up instructions.
- Background run transcript recovery.
- Durable chat history.
- Run replay cursor and terminal-state preservation.
- Crash-aware orchestration recovery.

## 2.4 Context engineering

Status: **ALTAI shipped / IsanAgent embedded**

- File mentions via `@`.
- Slash commands via `/`.
- Reusable snippets via `#`.
- Attach current Git diff.
- Attach terminal output.
- Image and PDF attachments.
- Project instructions from `ALTAI.md`.
- Workspace commands from `.altai/commands/*.md`.
- Tool-result pruning.
- Tunable automatic compaction.
- Manual `/compact`.
- Searchable aliases `/smol`, `/condense`, and `/summarize`.
- Section-aware summaries.
- Pre-summarization cleanup.
- Emergency compact-and-retry for context overflow.
- Window-aware compaction thresholds.
- Reversible and persistent tool-result swapping.
- Tool-result recall.
- Live context-window usage.
- Cost display.
- Anthropic prompt caching support.

## 2.5 Composer and chat

Status: **ALTAI shipped**

- Multi-line composer.
- File picker.
- Snippet picker.
- Slash-command search.
- Voice dictation via Whisper.
- Model dropdown.
- Agent switcher.
- Permission-mode switcher.
- Todo strip.
- Tool activity rendering.
- Tool approvals.
- Code, Markdown, and streamed response rendering.
- Session tabs.
- Chat history and rename.
- Background-task surface.
- Notification inbox.
- Automation surface.
- Run inspector.

## 2.6 Slash-command system

Status: **ALTAI shipped**

36 built-in commands:

### Session

- `/new` (`/clear`)
- `/sessions` (`/history`, `/resume`)
- `/rename`
- `/retry` (`/regenerate`)
- `/stop` (`/cancel`)
- `/compact` (`/smol`, `/condense`, `/summarize`)

### Workspace

- `/init`
- `/index` (`/map`)
- `/search` (`/find`)
- `/status` (`/activity`, `/inspect`)
- `/git-status` (`/git`)
- `/diff`

### Code

- `/plan` (`/architect`)
- `/explain` (`/ask`)
- `/fix` (`/debug`)
- `/refactor`
- `/todo` (`/checklist`)

### Quality

- `/test`
- `/lint`
- `/build`
- `/review`
- `/security`
- `/perf` (`/performance`)

### Project

- `/docs` (`/document`)
- `/workflow`
- `/research`
- `/paper`
- `/tasks`
- `/inbox`
- `/automations` (`/schedule`)

### Settings

- `/agents` (`/agent`)
- `/models` (`/model`)
- `/permissions` (`/permission`)
- `/mcp` (`/mcps`)
- `/skills`
- `/context`

### Dynamic commands

- Markdown files in `.altai/commands/*.md` become searchable project commands.
- YAML frontmatter can define command name, title, description, and aliases.
- Project commands cannot override built-ins.
- Project commands still pass through the normal agent permission flow.

## 2.7 Built-in agent personas

Status: **ALTAI shipped**

1. **Coder** — implementation, editing, and verification.
2. **Architect** — options, tradeoffs, risks, and plan-first design.
3. **Code Reviewer** — correctness, performance, security, race, and integrity
   review.
4. **Security** — threat modelling, exploitability, and systemic fixes.
5. **Designer** — hierarchy, spacing, density, contrast, motion, and states.
6. **Adaptive ML** — research, enumerate, pilot, evaluate, scale, verify,
   persist.
7. **Paper Reproducer** — paper-to-code and paper-to-notebook reproduction.
8. **Notebook Assistant** — focused data-science and notebook workflows.
9. **Dataset Generator** — Afterimage-backed synthetic-data production.

Additional behavior:

- built-ins can be renamed and instruction-overridden;
- built-ins can be disabled;
- custom agents can be created;
- active-agent preference is persisted;
- agent-specific icons and descriptions;
- IsanAgent routes the four ML-specialist personas.

## 2.8 Skills, MCP, and hooks

Status: **ALTAI shipped / IsanAgent embedded**

- Install skills from GitHub repositories.
- Install all skills or a selected skill from a repository.
- Load newly installed skills without restarting.
- Per-workspace skill catalogue.
- Per-workspace MCP configuration.
- Live MCP server status.
- MCP tool probing.
- Claude Desktop-compatible MCP configuration.
- Lifecycle hooks such as session start, before tool, after edit, and on error.
- Observation hooks with structured envelopes.
- Steering hooks that can alter or stop execution.
- Custom agent and workflow Markdown.

---

# 3. Multi-agent orchestration

Status: **ALTAI shipped**, with some deeper adapters still evolving

## 3.1 Task and workflow model

- Task board.
- Queued, running, reviewing, failed, and completed states.
- Task dependencies.
- DAG validation.
- Cycle detection.
- Topological scheduling.
- Worker pool.
- Up to eight parallel runners.
- Native runner adapter.
- Mock runner for validation.
- Plans and decomposition.
- Workflow configuration in `WORKFLOW.md`.
- Workflow v2 model.
- Layered configuration and config diff.

## 3.2 Agent profiles and teams

- Per-agent model.
- Reasoning effort.
- Permission mode.
- Tool allow/deny.
- Skills.
- MCP servers.
- Token, time, and cost budgets.
- File scopes.
- Environment selection.
- Team hierarchy.
- Agent-to-agent mailbox.
- Exactly-once coordination semantics.
- File-conflict detection.
- Integration coordinator.
- Routing rules.

## 3.3 Governance and quality

- Policy engine.
- Approval boundaries.
- Command quality gates.
- Automated checks.
- Pass/fail verification.
- Artifact and evidence store.
- Delivery gates.
- Budget accounting.
- Usage wiring.
- Quality scoring.
- First-attempt success, retry, and verification-failure metrics.
- Security boundary enforcement.
- Credential broker.

## 3.4 Operations

- Project-management sidebar.
- Orchestration bar.
- Orchestration control center.
- Run inspector.
- Activity stream.
- Durable SQLite ledger.
- Event projections.
- Run replay.
- Recovery.
- Notifications.
- Background jobs.
- Clarification tickets.
- Schedules and automations.
- Lifecycle hooks.
- Soak and failure testing support.

## 3.5 Environments and remote work

- Reproducible environment profiles.
- Local runner.
- Docker execution support.
- SSH execution path through IsanAgent.
- Remote worker-pool architecture.
- Browser QA module.
- Artifact collection.
- External source adapters.
- Local source and outbox source.
- Remote/mobile companion remains **roadmap** and must not be marketed as
  generally available.

## 3.6 Repository intelligence and learning

- Agent-readiness scan.
- Nine-dimension repository scoring.
- Evidence links for readiness findings.
- Context-pack builder.
- Session analysis.
- Execution plans.
- Decision logs.
- Repository gardening.
- Evaluation lab.
- Replay-based comparison.
- Smart routing.
- Quality dashboard foundations.
- Reusable playbooks and skills from repeated success.

---

# 4. IsanAgent — embedded ML runtime

## 4.1 Product model

Status: **IsanAgent embedded**

- Always-on agentic ML engineer.
- Outcome ownership across research, code, runs, checks, and handoff.
- Workspace-rooted operation.
- Local-first runtime.
- Terminal, HTTP API, and optional embedded web UI.
- Slack and email channel adapters.
- Multiple concurrent chats.
- FIFO task admission per chat.
- Run-scoped provider configuration.

## 4.2 Research

- Web search and fetch.
- arXiv search and fetch.
- Hugging Face Hub file retrieval.
- Workspace-memory search.
- Source-aware ML research overlay.
- Research-oriented subagents.
- Fresh-fact expectation before method selection.

## 4.3 Execution harness

- Capability-aware execution sessions.
- Local Python.
- System Python and UV-managed environments.
- Jupyter kernels.
- Notebook-aware framing.
- SSH execution.
- Colab MCP workflows.
- Background jobs.
- Job status, result, list, and cancel.
- Streaming logs.
- Execution manifests.
- Run telemetry.
- Run history.
- Preflight checks.
- Post-run handling.
- Auto-promotion of useful outputs.
- In-flight synchronization.
- Artifact capture and listing.
- Environment information.
- Session close and execution cancellation.
- Local/Jupyter/SSH provider selection.
- Long-running jobs without blocking the agent.

## 4.4 ML-engineering behaviors

- Eight-step Adaptive ML loop:
  understand, research, enumerate, pilot, evaluate, scale, verify, persist.
- Verifiable goal definition.
- Numeric method tradeoffs.
- Small pilots before scale.
- Pass criteria written before execution.
- Parallel independent pilots through subagents.
- Monitor agent for long-running scale jobs.
- Failure-class analysis.
- No silent lowering of the target.
- Doom-loop detection and strategy change.
- Budget-overrun stop.
- Repeated-success persistence as a skill.

## 4.5 ML domains represented in the runtime

- End-to-end PyTorch, JAX, and Flax workflows.
- Fine-tuning and post-training.
- PEFT: LoRA, QLoRA, DoRA, rsLoRA, LoftQ.
- Preference optimization and RL: DPO, KTO, ORPO, SimPO, IPO, GRPO, DAPO,
  RLOO, Online DPO.
- Quantization: AWQ, GPTQ, W8A8, FP8, GGUF, EXL3, HQQ, AQLM.
- Serving: vLLM, SGLang, LMDeploy, llama.cpp, Ollama, MLX-LM, ExLlama,
  TensorRT-LLM/NIM, ExecuTorch.
- Speculative decoding and multi-LoRA.
- RAG architecture, embeddings, vector stores, reranking, chunking, and
  late-interaction retrieval.
- Evaluation harnesses and modern benchmark selection.
- Synthetic data via Afterimage.
- Scientific Python debugging.
- Kernel optimization and porting.
- AutoTrainess post-training workflows.

## 4.6 Kernel porting

Status: **IsanAgent embedded / specialist**

- MaxEvolve kernel-porting workflow.
- Kernel-porting tools.
- Validator selection.
- Hardware-aware work.
- Benchmark-driven acceptance.
- Local or remote execution.
- JAX/Pallas, CUDA, and Triton-oriented workflows.

## 4.7 AutoTrainess

Status: **IsanAgent embedded / specialist**

- Post-training operator workflow.
- Named specialist agents.
- Project layout generation.
- Config-driven training backend.
- SSH-GPU preference for real training.
- Hard constraints and operator guidance.

## 4.8 Memory, reflection, and durable state

- SQLite memory actor.
- Durable conversations.
- Root-thread listing and previews.
- Notifications.
- Background jobs.
- Clarification tickets.
- Reflection pipeline.
- Memory search.
- Structured outcome persistence.
- Run completion and lifecycle events.
- Workspace-specific state under `.isanagent`.

## 4.9 Scheduling and channels

- One-time `at` schedules.
- Repeating `every` schedules.
- Cron expressions.
- Destination-aware scheduled work.
- Terminal channel.
- HTTP API.
- Embedded UI.
- Slack adapter.
- Email adapter.

## 4.10 Provider system

- Gemini.
- OpenAI.
- Anthropic.
- DeepSeek.
- OpenRouter.
- Multiple named providers in one configuration.
- Runtime `/model` switching.
- Persisted last model choice.
- In-flight run isolation when the model changes.
- Run-scoped failover snapshot.

## 4.11 Safety and resilience

- Workspace sandbox.
- Ignore rules.
- Provider redaction.
- Tool policy.
- Run budgets.
- Doom-loop detection.
- Checkpoints.
- Cancellation.
- Log rotation.
- Bounded tool-result handling.
- Context compaction and overflow recovery.
- Structured clarification.

---

# 5. Afterimage — synthetic dataset system

## 5.1 Generation

Status: **Afterimage available**

- Async conversation generator.
- Multi-turn synthetic conversations.
- Document-grounded Q&A.
- RAG-style context injection.
- Persona generation from text or documents.
- Persona-conditioned instruction generation.
- Structured-output generation.
- Tool-calling dataset generation.
- MCQ generation.
- Custom instruction-generator callbacks.
- Respondent prompt modifiers.
- Stopping callbacks.
- Per-turn hooks.
- Correspondent and respondent model separation.
- Different models for simulation and response.
- Sampling configuration.
- Generation orchestration.

## 5.2 Document and retrieval sources

- In-memory documents.
- Single files.
- Directories.
- JSONL.
- Qdrant-backed retrieval.
- Custom document providers.
- Context metadata and citations.
- Embedding providers.
- Local embedding optional extra.

## 5.3 Preference data

- DPO pairs.
- RLHF-oriented preference data.
- UltraFeedback-compatible output.
- Anthropic HH-compatible output.
- ORPO-oriented data.
- Temperature variation.
- Prompt variation.
- Model variation.
- Combined variation.
- Multi-turn preference pairs.
- Full generation logs.
- Preference analytics.
- Multiple quality criteria and judge strategies.

## 5.4 Evaluation and quality

- LLM-as-judge.
- Conversation judge.
- Coherence metric.
- Grounding metric.
- Relevance metric.
- Factuality metric.
- Helpfulness metric.
- Embedding-based evaluation.
- Composite quality scoring.
- Quality gates.
- Auto-improve retries.
- Threshold-based rejection.
- Evaluation strategies and extension points.

## 5.5 Scale and key management

- Async-first execution.
- Configurable concurrency.
- Smart API-key pool.
- Automatic key rotation.
- Per-key rate limits.
- Concurrent request management.
- Incremental writes.
- Crash-safe resumability.

## 5.6 Storage

- JSONL default storage.
- SQLite.
- PostgreSQL.
- MySQL.
- Custom storage implementation.
- Incremental storage during generation.

## 5.7 Export and delivery

- ShareGPT.
- Alpaca.
- Hugging Face Messages.
- LLaMA Factory.
- Oumi.
- OpenAI fine-tune.
- DPO.
- Raw output.
- Multiple export formats in one command.
- Automatic train/validation split.
- Programmatic exporter registry.
- Push to Hugging Face Hub.
- Integrations with Unsloth, Axolotl, TRL, LLaMA Factory, Oumi, and OpenAI
  fine-tuning.

## 5.8 Observability

- Generation monitor.
- Real-time metrics.
- JSON metrics export.
- CSV export.
- Excel multi-sheet export.
- Parquet export.
- Time-window filtering.
- Standard plots.
- Configurable alerts.
- Custom alert handlers.
- Thread-safe handlers.
- Dataset analytics engine.
- HTML analytics report generation.
- Logs under an Afterimage monitoring directory.

## 5.9 Interfaces and packaging

- YAML configuration.
- Configuration validation.
- Dry run.
- CLI.
- Python API.
- FastAPI server.
- SSE progress streaming.
- Gradio demo UI.
- Python 3.11+.
- Optional extras for server, local embeddings, and training.
- Local OpenAI-compatible providers.

## 5.10 Model providers

- Google Gemini.
- OpenAI.
- DeepSeek.
- OpenRouter.
- Local OpenAI-compatible servers.
- vLLM.
- Ollama.
- llama.cpp.
- Provider extensibility.

## 5.11 OpenSimula

Status: **Afterimage available / experimental**

- Factor taxonomy generation.
- Taxonomy bundles.
- Weighted factor-mix sampling.
- Sampling strategy persistence.
- Meta-prompt diversification.
- Optional prompt complexification.
- Requirement critics.
- Critic-driven refinement.
- Independent double-critic gate.
- Verifiable multiple-choice pipelines.
- Checkpoint manifests.
- JSONL sample streaming.
- Dataset-card support.
- Shared `GenerationMonitor`.
- Bridge into `ConversationGenerator`.
- Document-context support.
- Evaluation utilities.
- Scenario export.

## 5.12 Context to skill

Status: **Afterimage available / new**

- Convert large contexts into agent skills.
- Generate skill proposals.
- Probe generation.
- Skill judging.
- Candidate selection.
- Iterative optimization.
- Store skill-generation state.
- Compare baseline, original ctx2skill, and Afterimage variants.
- Evaluate pass rates.
- Produce reusable skill text for agent workflows.

---

# 6. Product claims to avoid

- Do not claim a hosted cloud-agent service is generally available.
- Do not claim the mobile companion is shipped.
- Do not imply every Afterimage capability has a dedicated graphical control
  in ALTAI; many are agent-operated through the runtime and Python library.
- Do not claim all deep orchestration modules are equally mature.
- Do not promise a benchmark win without a current, reproducible source.
- Do not present saturated benchmarks as headline proof.
- Do not call ALTAI “no-code.” It can serve non-experts, but its core value is a
  real, inspectable engineering environment.
- Do not claim privacy merely because the app is local-first; explain keys,
  provider calls, boundaries, and local-model options precisely.
