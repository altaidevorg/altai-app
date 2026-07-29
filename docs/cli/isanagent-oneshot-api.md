# IsanAgent oneshot host API (M1)

**Status:** Temporary path pin pending upstream merge  
**Base revision:** `8c9eef2cf63f0b888f5b73778f056ba7d21467cf`  
**Local pin:** `tools/isanagent-oneshot`  
**Diff artifacts:** `docs/cli/isanagent-oneshot-api.*.diff`

## Public additions

```rust
isanagent::host::HostConfig {
    // existing fields...
    pub oneshot_prompt: Option<String>,
    pub observe_tx: Option<mpsc::UnboundedSender<BusMessage>>,
    pub scripted_responses: Option<Vec<String>>,
    /// ALTAI terminal appearance. `no_color` / `NO_COLOR` still win.
    pub theme: HostThemeMode, // Auto | Dark | Light | NoColor
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

## Upstream constraint

`gh` auth in this environment is invalid, so the change could not be pushed as an
IsanAgent PR during M1. ALTAI temporarily path-depends on `tools/isanagent-oneshot`.
After the upstream PR merges, switch `src-tauri/Cargo.toml` and
`src-tauri/crates/altai-cli/Cargo.toml` back to the git revision and delete the
local pin.
