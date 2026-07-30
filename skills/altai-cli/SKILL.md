---
name: altai-cli
description: Use the ALTAI terminal product to run agents, inspect durable workspace state, and automate safe coding workflows through human-readable or JSONL output.
---

# ALTAI CLI

The CLI is under active development. Its production contract is defined in
`docs/plans/2026-07-29-complete-cli.md`.

## Intended command families

```text
altai agent [PATH]
altai run [PATH] --prompt TEXT --output jsonl
altai chat ...
altai jobs ...
altai inbox ...
altai skills ...
altai mcp ...
altai orchestration ...
```

## Agent usage rules

- Prefer `--output jsonl` for programmatic workflows and parse stdout only.
- Inspect state before mutations when a `list`, `show`, `status`, or `--dry-run`
  option exists.
- Never pass API keys as command-line arguments.
- Treat approval-required and non-TTY outcomes as actionable failures, not as
  implicit permission to bypass safeguards.
- Use `altai open` only when a visual-only desktop surface is needed.

