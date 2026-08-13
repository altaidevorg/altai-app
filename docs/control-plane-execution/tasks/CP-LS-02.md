# Task Packet: CP-LS-02 — Local SQLite Scope, Agent and Work Graph Adapters

```yaml
task_id: CP-LS-02
title: Persist scope, agent registry and Work graph control-plane ports in the workspace work.db
risk_tier: B
parent_module: CP-LS
status: in_progress
depends_on: [CP-LS-01]
objective: |
  Complete desktop-local persistence consolidation by wiring durable scope,
  agent registry and Work graph adapters to the same workspace work.db.
non_goals:
  - A second local database, user-managed Postgres, Docker, or PGlite
  - A remote multi-machine API or backend
  - Wake/lease claim semantics
invariants:
  - work.db remains the sole desktop durable Work OS store.
  - Organization boundaries, hierarchy cycles and agent reporting cycles fail closed.
  - Existing authenticated routes use the local durable adapters.
tests_required:
  - cargo test --manifest-path src-tauri/Cargo.toml -p altai-control-plane
  - cargo check --manifest-path src-tauri/Cargo.toml -p altai-control-plane
acceptance:
  - Scope, agent and Work graph data survive reopening workspace work.db.
  - The daemon supplies all three SQLite adapters to its router.
```
