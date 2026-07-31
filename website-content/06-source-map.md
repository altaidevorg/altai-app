# Source Map and Claim Hygiene

Snapshot date: 2026-07-29

## Repository revisions

### ALTAI

- Repository: [altaidevorg/altai-app](https://github.com/altaidevorg/altai-app)
- Revision: `0df46ee5b3335bd850f00ff59801b8e2bc089a91`
- Release metadata: `0.6.5`
- Local worktree contained pre-existing, uncommitted UI changes during the
  audit. The content pack did not modify those files.

### IsanAgent

- Repository: [altaidevorg/isanagent](https://github.com/altaidevorg/isanagent)
- Default branch: `main`
- Current revision resolved during research:
  `f40a5a48b120070b23a913f7637e8f89a71da540`
- The ALTAI Cargo dependency tracks the `main` branch.
- Release workflow runs `cargo update -p isanagent` before building so the
  current branch tip is resolved.

### Afterimage

- Repository: [altaidevorg/afterimage](https://github.com/altaidevorg/afterimage)
- Default branch: `main`
- Current revision resolved during research:
  `ed59793d591eb2a009d6364424259a7c0fb42a77`
- Documentation: [afterimage.altai.dev](https://afterimage.altai.dev/)
- Package: [PyPI](https://pypi.org/project/afterimage/)

## ALTAI primary sources scanned

- `README.md`
- `INSTALL.md`
- all Markdown under `docs/`
- `.altai/agents/`
- AI components, stores, libraries, and built-in agent definitions
- slash-command registry
- editor, terminal, notebook, preview, Git, GitHub, MCP, LSP, source-control,
  settings, accessibility, and workspace modules
- Rust filesystem, process, PTY, Git, GitHub, notebook, MCP, webview, secret,
  network, and workspace modules
- all 44 source files under the Rust orchestration module
- current package and Tauri metadata

## IsanAgent primary sources scanned

- `README.md`
- `AGENTS.md`
- execution user guide
- execution use cases
- hooks guide
- kernel-porting guide
- AutoTrainess operator guide
- public API surface
- source-module and tool structure
- current default-branch tree

## Afterimage primary sources scanned

- `README.md`
- `AGENTS.md`
- `DESIGN.md`
- architecture and overview docs
- conversation, structured, persona, evaluation, monitoring, preference, local
  model, export, and advanced-usage docs
- Context-to-Skill tutorial
- OpenSimula docs and source tree
- provider, evaluator, quality, storage, analytics, server, skill, preference,
  exporter, and test structures
- current default-branch tree

## Competitive primary sources

- [Cursor](https://cursor.com/)
- [Warp](https://www.warp.dev/)
- [Claude Code](https://claude.com/product/claude-code)
- [Kilo Code](https://www.kilocode.app/)
- [Kilo repository](https://github.com/Kilo-Org/kilocode)
- [Cline](https://cline.bot/)
- [Zed AI](https://zed.dev/ai)
- [Replit Agent](https://replit.com/products/agent)
- [Bolt](https://bolt.new/)
- [Lovable](https://lovable.dev/)
- [v0](https://v0.app/)
- [Devin documentation](https://docs.devin.ai/get-started/devin-intro)

## Claim rules for future updates

1. Resolve IsanAgent and Afterimage’s current default-branch revisions before
   updating feature claims.
2. Do not rely on an older clone, package version, or cached README.
3. Verify ALTAI-facing UI claims in `altai-app`, not only in IsanAgent or
   Afterimage.
4. Label specialist or experimental subsystems.
5. Separate “agent can invoke this library” from “the app has a dedicated
   graphical screen for this.”
6. Keep roadmap content out of the shipped-feature page.
7. For performance, benchmark, adoption, or pricing claims, add a dated primary
   source.
8. Re-check the interactive replica against the live ALTAI surface whenever
   the relevant product UI changes materially.
