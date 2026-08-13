# Task Packet: CP-LS-01 — Local SQLite Registration Consolidation

```yaml
task_id: CP-LS-01
title: Remove desktop Postgres control-plane configuration and persist registration in work.db SQLite
risk_tier: B
parent_module: CP-LS
status: in_progress
depends_on: [CP-07-01]
objective: |
  Make the existing workspace work.db the only desktop durable Work OS store.
  Replace the Postgres registration adapter and daemon URL configuration with a
  SQLite adapter that atomically consumes one-time grants and writes hosts.
non_goals:
  - Adding a multi-machine backend or remote API
  - Moving scope, agent registry, or Work graph persistence (CP-LS-02)
  - Wake claim/expiry semantics
allowed_files:
  - src-tauri/crates/altai-control-plane/**
  - src-tauri/Cargo.lock
  - docs/control-plane-execution/**
  - docs/adr/0003-control-plane-execution-plane-split.md
read_first:
  - docs/control-plane-execution/WORK_OS_PROGRAM_BACKLOG.md
  - docs/control-plane-execution/DECISIONS.md
  - src-tauri/crates/altai-core/src/workspace.rs
  - src-tauri/crates/altai-control-plane/src/service.rs
invariants:
  - Desktop users need no Postgres server, Docker, URL, or credential.
  - work.db is the sole local durable Work OS store.
  - Grant consumption and host registration are one SQLite transaction.
  - A remote multi-machine system is future ALTAI-managed backend work, not desktop configuration.
commands: []
events: []
migrations: [idempotent registration tables in existing work.db]
tests_required:
  - cargo test --manifest-path src-tauri/Cargo.toml -p altai-control-plane
  - cargo check --manifest-path src-tauri/Cargo.toml -p altai-control-plane
acceptance:
  - Postgres dependency and desktop Postgres URL are absent
  - Daemon resolves one workspace work.db path
  - SQLite transaction test proves a grant cannot be reused
stop_conditions:
  - Any change requires a user-managed database service
```
