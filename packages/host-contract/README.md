# `@altai/host-contract`

Product-neutral host ports and capability schema for ALTAI Desktop and the
VS Code extension.

- No `@tauri-apps/*` or `vscode` dependencies.
- Shared UI enables controls only when a capability is `available`.
- Wire JSON-RPC framing stays in `@altai/agent-protocol`; this package owns
  UI-facing ports and the capability document.

```bash
pnpm --filter @altai/host-contract typecheck
pnpm --filter @altai/host-contract test
```
