# IsanAgent oneshot host API (M1)

**Status:** Merged upstream — ALTAI tracks `altaidevorg/isanagent` `main`  
**Merged via:** [`altaidevorg/isanagent#101`](https://github.com/altaidevorg/isanagent/pull/101) (`ea90fa0`)  
**Historical base (path pin era):** `8c9eef2cf63f0b888f5b73778f056ba7d21467cf`  
**Diff artifacts (historical):** `docs/cli/isanagent-oneshot-api.*.diff`

## Public additions

```rust
isanagent::host::HostConfig {
    // existing fields...
    pub oneshot_prompt: Option<String>,
    pub observe_tx: Option<mpsc::UnboundedSender<BusMessage>>,
    pub scripted_responses: Option<Vec<String>>,
    /// ALTAI terminal appearance. `no_color` / `NO_COLOR` still win.
    pub theme: HostThemeMode, // Auto | Dark | Light | NoColor
    pub compact_auto: Option<bool>,
    pub compact_threshold_tokens: Option<usize>,
    pub compact_tail_turns: Option<usize>,
}

pub async fn run_oneshot(config: HostConfig) -> HostResult<OneshotResult>;
pub struct OneshotResult { chat_id, run_id, outcome, final_text }
pub enum OneshotOutcome { Completed, Failed, Cancelled, TimedOut, ApprovalRequired, ClarificationRequired }
pub use HostThemeMode;
```

## Behavior

- When `oneshot_prompt` is set, the interactive terminal is disabled.
- A headless `altai-cli` channel injects one inbound prompt and captures the final outbound.
- `RunLifecycle::Terminated` completes the oneshot and shuts the host down.
- `approval_requested` / clarification outbound ends with exit-friendly outcomes (no silent approve).
- `scripted_responses` provides an in-process mock provider for deterministic CI/smoke tests.
- Startup banners for oneshot mode go to stderr so JSONL stdout stays clean.
- `theme` selects ALTAI dark/light truecolor roles (or no-color structure); combined with
  `no_color` before the Ratatui / line-mode channel starts.
- Approval prompts offer `approve` / `deny` / `always` / `abort`. `always` grants for the
  current process only. Edit approvals attach `metadata.edit_diff` and map oneshot
  completion to `ApprovalRequired` with the diff detail.
- `compact_*` host fields override memory compaction thresholds; `compact_auto =
  Some(false)` disables between-turn auto-compaction (`short_term_threshold_tokens =
  usize::MAX`) while manual `/compact` still works.
- Oneshot `--file` paths load through `load_host_file_attachments` (text wrapped as
  `<context-file>`, images/PDFs as media parts). Terminal `@path` parsing accepts
  text / image / PDF and fuzzy-resolves unique basenames under the sandbox.

## Upstream status

Host oneshot / theme / approval / attachment APIs landed on IsanAgent `main` via
[#101](https://github.com/altaidevorg/isanagent/pull/101) (superseding the earlier
[#99](https://github.com/altaidevorg/isanagent/pull/99) / revert cycle). ALTAI
depends on git `main` again; the temporary `tools/isanagent-oneshot` path pin is
removed. `pnpm isanagent:sync` / CI continue to refresh the lockfile tip.

## M5 / M6 notes

- Shared durable journal lives in `altai-core::journal` (not this host surface).
- Oneshot resumes approvals on `/dev/tty` when available; otherwise exits with
  `ApprovalRequired` / `ClarificationRequired` for non-TTY callers.
