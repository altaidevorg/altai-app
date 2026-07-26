# ALTAI repository rules

- Preserve unrelated user changes in the working tree.
- Search with `rg` and `rg --files`.
- Make focused edits; do not perform broad mechanical rewrites without need.
- Do not use destructive Git commands.
- Do not push, merge, open a PR, or modify remote state unless explicitly asked.
- Reuse the existing runtime, event, state, and component patterns.
- Keep GitHub optional; local workflows must continue to work offline.
- For orchestration, treat SQLite and the native coordinator as authoritative.
- Add deterministic tests for state transitions, recovery, and failure paths.
- Read only the named roadmap section and directly related code for each task.
- Report changed files, verification, remaining risks, and the next roadmap item.
