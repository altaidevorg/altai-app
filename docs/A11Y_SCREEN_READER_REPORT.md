# ALTAI — Full Screen-Reader Compatibility Report

> Follow-up to `docs/A11Y_AUDIT.md` (2026-05-22). Scope: what it takes for ALTAI
> to be **fully** usable with VoiceOver / NVDA / JAWS / Orca, on `main` after the
> native-runtime migration (PR #24/#25). Audited 2026-06-02 by four parallel
> a11y passes (regression verification, new chat surfaces, deep panes, systemic
> infrastructure). Code not modified by this report.

---

## Executive summary

The original 3-sprint plan **landed almost entirely** — every Critical (C1–C10)
and High (H1–H8) finding from the first audit is fixed in `main`, plus the
Accessibility settings panel, skip-links, focus-restore, and reduced-motion are
real and wired. A blind developer can use the **core chat + terminal + explorer
+ source-control** flows today.

"**Fully** compatible" is still blocked by four classes of gap:

1. **Systemic infrastructure** that no per-component fix covers — most notably
   **zero Windows High-Contrast / `forced-colors` support** and **no automated
   a11y testing** to keep compliance from silently regressing.
2. **New surfaces added after the first audit** (workspace welcome, clarification
   chips, inline tool cards, inline status pill) that were never audited.
3. **Deep panes** the first audit deferred — **notebook** and **diff** are the
   weakest; editor and terminal are largely fine.
4. A recurring **"placeholder-as-label"** regression — 8 new text inputs ship
   unlabeled, the same failure C6/C7 fixed for the chat/commit boxes.

Plus a hard truth: the upcoming **token-streaming** work (`next.md`) will, with
the current live-region setup, make the screen reader **stutter every token** —
it must be designed correctly *before* streaming ships.

---

## Part A — Regression status of the original audit (✅ mostly holds)

All verified against current `main`.

| Finding | Status |
|---|---|
| C1 terminal `screenReaderMode` | ✅ wired to pref (`rendererPool.ts:99`, default on `store.ts:197`) |
| C2 terminal container role/label | ✅ `TerminalPane.tsx:73-78` |
| C3 chat log `aria-live` | ✅ `conversation.tsx:19-23`, pref-driven via `chatAnnounce` |
| C4 approval live region | ✅ `AiToolApproval.tsx:48-54` |
| C5 status pill `role="status"` | ✅ `AgentStatusPill.tsx:57` |
| C6/C7 composer + commit labels | ✅ `AiInputBar.tsx:340`, `SourceControlPanel.tsx:568` |
| C8 close-tab real button | ✅ `TabBar.tsx:130-137` |
| C9 AI session tablist | ✅ `AiSidePanel.tsx:186-189` (close btn now 24px ✓) |
| C10 history-row keyboard | ✅ `ChatHistory.tsx:304-316` |
| H1/H8 icon-button labels | ✅ swept |
| H2 `<Label htmlFor>` | ✅ promoted to `<h3>` headings |
| H3 chat error `role="alert"` | ✅ `AiChat.tsx:247` |
| H5 explorer tree semantics | ✅ `FileExplorer.tsx:446`, `TreeRow.tsx:98-101` |
| H6/H7 landmarks + skip-links | ✅ `App.tsx:1432-1440`, `Header.tsx`, `SidebarRail.tsx` |
| M3 reduced-motion | ✅ `globals.css:330` + pref |
| M6 Escape guard | ✅ `AiSidePanel.tsx:58-69` |
| A11y settings panel + appliers | ✅ all 8 prefs have runtime consumers |

**Still open / partial from the original audit:**

- **M1 ❌** light-theme `--muted-foreground: oklch(0.56)` still fails AA 4.5:1
  for secondary text (`globals.css:66`). Only the *opt-in* high-contrast override
  was darkened. Default light theme remains non-compliant.
- **H4 ⚠️** `PaperImport.tsx:71` input still placeholder-only (no label).
- **M4 ⚠️** settings have `<h3>` now, but no `<h1>` session title / no broad
  heading hierarchy in the chat panel.
- **M5 ⚠️** `ChatHistory` `RowIconButton` still 20px (`size-5`), below 24px.
- **M2 ⚠️** state-carrying tab borders still low non-text contrast.

---

## Part B — New chat surfaces (post-migration, never audited)

| ID | Severity | Surface | Symptom & fix |
|---|---|---|---|
| N1 | **Critical** | `WorkspaceGate`/`WorkspaceWelcome.tsx:72` | Gate renders with **no landmark and no initial focus** → SR user lands on `<body>`, contextless. Wrap in `<main aria-labelledby>`, make title `<h1>`, move focus to it on mount. |
| N2 | **High** | `AgentStatusPill.tsx` (inline) | The pill now mounts **twice** (inline transcript + log opener) → **two `role="status"` live regions double-speak every status change**. Hoist to a single owner / gate the sr-only region to one instance. |
| N3 | **High** | `tool.tsx:303` | Inline tool cards **don't announce status transitions** (running→done→**error**). Status dot is a static `<span>`. Wrap a visually-hidden mirror in `role="status" aria-live="polite"`; give `is_error` an assertive marker so failed tools announce as errors. |
| N4 | **High** | `ClarificationChoices` (`AiSidePanel.tsx:371`) | `ask_user` preset-answer chips appear **silently** (no live region). Add `role="group"` + `aria-label` and a polite "N suggested replies" announcer. (Chips themselves are correct real buttons.) |
| N5 | High | `WorkspaceWelcome.tsx:126` | Clone **error** not announced (`role="alert"` missing) and clone **progress** ("Cloning…") not in a live region. |
| N6 | Medium | `WorkspaceWelcome.tsx:155` | "Recent" is a styled `<div>`, not a heading; list not associated. Promote to `<h2>` + `aria-labelledby`. |

**Already correct (good):** `CommandGroup` / read / web collapsibles use Radix
`Collapsible` → native `aria-expanded`/`aria-controls`, keyboard-operable, sensible
"Ran N commands" labels. Clone URL input is labeled. Recents are real `<ul>/<li>`
with labeled remove buttons.

---

## Part C — Deep panes (terminal / editor / diff / notebook / preview)

- **Terminal ✅ (mostly).** `screenReaderMode` correct; WebGL does **not** defeat
  SR (xterm builds a separate `.xterm-accessibility` DOM tree). Inherent limits:
  `role="application"` drops virtual-cursor browse mode while focused (F1, expected);
  scrollback above the xterm row cap isn't announced (F2, inherent — mitigate with
  an "announce visible buffer" keybinding).
- **Editor (CodeMirror) ⚠️.** Stock CM6 a11y is preserved (textbox role, completion
  listbox, search reachable) but the editor has **no accessible name** — SR says
  "edit text" with no file context (F4). Add `aria-label="Editor: <path>"` via
  `EditorView.contentAttributes`.
- **Diff panes 🔴 (High, F5).** Added/removed lines are conveyed by **background
  color only** (`AiDiffPane.tsx:64`, `GitDiffPane.tsx:49`) and the `+N/−M` stats are
  unlabeled spans — **WCAG 1.4.1 fail**. Add text labels to stats; expose
  added/removed line semantics (or offer the `pre` `+`/`-` view as an SR mode).
- **Notebook 🔴 (two High).** Cell toolbar is `hidden group-hover:flex` →
  **Run/Move/Delete are unreachable by keyboard/SR** (F7); execution **outputs are
  not announced** (no `aria-live`, F8). Also no cell list semantics (F6), useless
  `alt="output"` on images, raw `dangerouslySetInnerHTML` outputs (F9), and the
  markdown cell enters edit mode via a non-keyboard `<div onClick>` (F10b).
- **Preview ✅.** iframe has a `title`; minor: make it URL-specific, label the
  reload button (F10).

---

## Part D — Systemic / infrastructure gaps (the real blockers for "fully")

1. **No `forced-colors` / OS high-contrast support — HIGHEST, fully missing.**
   Zero `@media (forced-colors: active)` anywhere; the OKLCH theme is stripped under
   Windows High-Contrast with no `SystemColor` fallbacks or border re-assertion →
   icon-only controls become invisible. The app's high-contrast pref is opt-in and
   does **not** track the OS (`prefers-contrast`). *Add a forced-colors stylesheet +
   honor `prefers-contrast`.*
2. **No automated a11y testing / lint gate — High (regression risk).** No
   `eslint-plugin-jsx-a11y`, `vitest-axe`, or `@axe-core/playwright`; no ESLint
   config at all. "Fully compatible" cannot *stay* true without this. *Add jsx-a11y
   lint (CI gate) + vitest-axe component smoke tests + one Playwright axe window scan.*
3. **Streaming will spam the SR — must fix before `next.md` ships.** Current
   `role="log"` + `aria-live="polite"` + `aria-relevant="additions text"` announces
   **every token**. Switch to `aria-busy="true"` on the container during stream +
   `aria-relevant="additions"` only, and announce the **finalized** message once via
   a dedicated `aria-atomic` polite region on the `done` event. (`message.tsx` already
   sets `aria-busy` on the streaming part — extend it.)
4. **Focus orphaned on view changes — High.** AI panel **close** doesn't restore
   focus to the opener (`closeMini`, `chatStore.ts:287`); **workspace tab switch**
   never moves focus into the newly-shown surface (`App.tsx:1445`, panes toggled via
   `aria-hidden`); folder-close/diff-open don't place focus. Capture-and-restore +
   move-focus-into-new-region.
5. **No app-wide live region for non-chat async results.** Updater **progress** is
   silent (no `role="progressbar"`/`aria-valuenow`); some results use `window.alert`;
   no toast/`role="status"` surface for "file written", "command done", auth failures.
   *Add one global polite live region + a correct progressbar.*
6. **Document/window polish.** No dynamic `document.title` (window list/title is
   static "ALTAI"); the **settings window** lacks an `<h1>`/heading landmark; the
   workspace tab strip isn't wired as `tablist`/`tabpanel` with `aria-labelledby`.
7. **Reflow at high zoom (1.4.10) — desktop caveat.** Fixed-px three-panel shell +
   globally suppressed scrollbars + `overflow:hidden` can't reflow to one column at
   400%; arguable desktop exception, but the **chat panel** itself should reflow and
   overflowed content must stay reachable.

---

## Part E — Net-new "placeholder-as-label" regressions

Same failure mode C6/C7 fixed, now recurring on newer inputs (all need an
`aria-label` or associated `<label>`):

1. `ExplorerSearch.tsx:184` — explorer search
2. `ChatHistory.tsx:219` — history search
3. `NotebookCell.tsx:93` — cell source `<textarea>`
4. `NewEditorDialog.tsx:93` — filename input
5. `PaperImport.tsx:71` — URL input (H4)
6. `AiStatusBarControls.tsx:329` — model search
7. `PreviewAddressBar.tsx:179` — URL bar
8. `SearchInline.tsx:160` — header search (audit had wrongly assumed labeled)

Plus: `CwdBreadcrumb.tsx:272` "Show hidden folders" toggle is icon-only with
`title` only (needs `aria-label` + `aria-pressed`); `NotebookCell.tsx:89` markdown
edit is a `<div onClick>` with no role/tabIndex/onKeyDown.

---

## Prioritized roadmap to "fully screen-reader compatible"

**P0 — systemic, blocks the claim:**
- Forced-colors / OS high-contrast stylesheet + honor `prefers-contrast` (§D1).
- Automated a11y CI: jsx-a11y lint + vitest-axe + Playwright axe scan (§D2).
- Streaming live-region strategy — design before `next.md` streaming lands (§D3).
- Focus management on panel-close / tab-switch / folder-close (§D4).

**P1 — high-impact component gaps:**
- Notebook: focusable toolbar + announced outputs + cell semantics (F6–F10).
- Diff: non-color conveyance + labeled stats (F5).
- Label the 8 unlabeled inputs + the hidden-folders toggle (Part E).
- Tool-card status live region incl. `is_error` (N3); AgentStatusPill single live region (N2); WorkspaceGate focus+landmark (N1); ClarificationChoices announcer (N4).

**P2 — medium polish:**
- Editor accessible name (F4); M1 light-theme contrast; M4 headings / `<h1>`;
  M5 24px targets; clone error/progress (N5/N6); updater progressbar + global live
  region (§D5); dynamic `document.title` + settings-window heading (§D6).

**P3 — inherent / desktop caveats:**
- xterm scrollback announce keybinding (F2); CodeMirror navigation caveats;
  reflow at extreme zoom (§D7, document as desktop exception, fix chat-panel reflow).

**Out of scope still:** RTL/locale, Talon/Dragon/Voice-Control flows, plugin
surfaces.
