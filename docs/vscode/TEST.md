# ALTAI VS Code test matrix

Status labels: **planned** means no corresponding VS Code test exists yet;
**baseline evidence** identifies an existing repository test or implementation
that the later task must preserve. No row is release evidence until its named
command runs in CI for the applicable target.

| ID | Task | Test / acceptance gate | Level | Status / owner |
| --- | --- | --- | --- | --- |
| VSC-00-docs | TVS-00 | ADR decisions, capability mapping, and named test coverage are reviewed; no production source changes | documentation | baseline evidence / Architecture owner |
| VSC-01-frame | TVS-01 | Rust and TypeScript accept golden fixtures; reject malformed headers, limits, missing ids/version/identity; forward-compatible optional fields | unit + fixtures | completed / Protocol owner; shared golden fixtures verify both implementations |
| VSC-02-stdio | TVS-02 | Spawn compiled CLI; split headers/bodies, multiple frames/read, EOF, invalid version, ordered start/event/terminal, malformed JSON close, and cancel terminality | CLI integration | completed / CLI owner; `altai-cli/tests/serve_stdio.rs` is the subprocess acceptance gate |
| VSC-03-event-seam | TVS-03 | Desktop envelope remains JSON equivalent; service has no Tauri dependency; sink failure and journal classification are safe | Rust contract | completed / Service owner; `altai-agent-service/tests/event_contract.rs` covers envelope JSON, terminal sink failure/retry, and restart classification |
| VSC-04-lifecycle | TVS-04 | Concurrent fingerprint isolation; stale cancel/steer rejection; session aliases; ordered replay without duplicates | Rust service + Desktop regression | planned / Service owner |
| VSC-05-host | TVS-05 | Two chats, reconnect replay, duplicate request id, stderr secret/header redaction, bounded backpressure, SIGINT shutdown | compiled CLI integration | planned / CLI owner |
| VSC-06-activation | TVS-06 | Activation starts no process; first action starts one exact canonical-folder host; isolated multi-root managers; trust/virtual gates; bounded deactivation | extension unit + `@vscode/test-electron` | completed / Extension owner; unit coverage is in `extensions/vscode/src/test`; Extension Development Host smoke remains a release gate |
| VSC-07-chat | TVS-07 | Session restore, webview reload, extension-host reload/replay, unknown optional event, CSP/message validation, keyboard/ARIA/theme smoke | extension unit + integration | planned / Extension owner |
| VSC-08-context | TVS-08 | Selection/file/diagnostic/editor/Git/image/PDF limits; removable chips; multi-root selection; remote/binary/deleted/symlink/oversize failure; injection-safe payloads | extension unit + integration | planned / Extension owner |
| VSC-09-approval | TVS-09 | Deny makes no mutation; stale diff invalidates approval; approve only reviewed operation; duplicate response handling; checkpoint create/modify/delete; trust/path confinement | Rust + extension integration | planned / Service + Extension owners |
| VSC-10-work | TVS-10 | One run id across Chat/Work/Inbox; projections deduplicate; Inbox resolution preserves history; reload replay has no duplicates; foreground limit is labeled | extension + service integration | planned / Work-surface owner |
| VSC-11-package | TVS-11 | Per-target VSIX has one correct binary; allowlisted archive, permissions/suffix, checksums/SBOM/license/provenance, architecture recovery, secret/dependency scan | package + CI | planned / Release owner |
| VSC-11-remote | TVS-11 | Local macOS/Windows/Linux and WSL/Remote SSH/Dev Container smoke prove the host runs where the workspace lives | platform CI | planned / Release owner |
| VSC-12-beta | TVS-12 | Clean-machine plan run; upgrade/rollback; accessibility and performance evidence; missing/incompatible/crashed-host recovery; privacy/onboarding review | release validation | planned / Release + Product owners |

## Required commands by phase

TVS-00 focused verification:

```bash
git diff --check -- docs
rg -n "TBD|TODO|open question" docs/adr docs/vscode
```

TVS-01 through TVS-12 must run the exact task-specific commands in
`docs/plans/2026-07-30-vscode-extension-terra-execution.md` before handoff.
The following existing baseline remains a regression reference, not a
substitute for a new gate:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml -p altai-core -p altai-cli
cargo clippy --manifest-path src-tauri/Cargo.toml -p altai-core -p altai-cli -- -D warnings
pnpm exec tsc --noEmit
pnpm lint
pnpm test
pnpm build
```

## TVS-02 stdio behavior

`altai-cli serve --stdio --protocol 1 --workspace <path>` reserves stdout for
LSP-style `Content-Length` frames for the process lifetime. Test-only scripted
responses require the debug-only `ALTAI_CLI_TEST_SCRIPTED_RESPONSE` environment
variable; release builds do not include this hook. Invalid protocol requests
that include an id receive a typed JSON-RPC error response. A malformed frame
or malformed JSON body has no safely recoverable request id, is recorded as a
single `altai-cli serve: malformed ...` stderr line, and closes the connection.

The TVS-02 integration suite uses the compiled `CARGO_BIN_EXE_altai-cli` binary
and proves split header/body delivery, multiple request frames in one write,
EOF shutdown, ordered `run_started` + assistant/tool activity + exactly one
terminal event, and cancellation terminality. The short debug-only terminal
pause is solely a deterministic cancellation-window hook; production behavior
and existing CLI JSONL output are not routed through it.

## Open questions

| Question | Owner | Blocking? | Resolution point |
| --- | --- | --- | --- |
| Protocol v1 size limits, typed error codes, and attachment representation | Protocol owner | Blocking for TVS-01 | Golden fixtures and schema review |
| Exact host-neutral replacement/alias strategy for `tauri:<chat_id>:` identities | Service owner | Blocking for TVS-04 | Desktop/service lifecycle contract |
| Tauri-independent secret facade and provider-auth UX | Security owner | Blocking for TVS-05 production credential path; non-blocking for scripted TVS-02 | Security ADR/implementation review |
| Marketplace publisher, code-signing, SBOM/provenance toolchain, and binary distribution owner | Release owner | Blocking for TVS-11/12, non-blocking for local development | Packaging design review |
| Supported target execution evidence for Linux arm64 and Windows arm64 | Release owner | Blocking for public beta target claim | CI matrix approval |
| Remote SSH, WSL, Dev Container, and Codespaces support commitment after Linux host validation | Product + Release owners | Blocking for public support claim, non-blocking for local shell | TVS-11 smoke evidence |
