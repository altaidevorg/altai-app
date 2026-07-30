# ALTAI Website Content Pack

This folder contains implementation-ready content for a future ALTAI product
website. It is intentionally not a website build.

## Core positioning

**ALTAI is the open agentic development environment for code and machine
learning.**

The product story should not present coding, ML engineering, and dataset
generation as three unrelated feature groups. The differentiator is the
continuous workflow:

> Understand the workspace → research → plan → edit → execute → evaluate →
> review → ship → remember what worked.

ALTAI provides the native workspace and governance layer. IsanAgent provides
the always-on ML execution and research runtime. Afterimage provides
production-grade synthetic data and dataset-quality workflows.

## Files

- `01-page-copy.md` — complete English landing-page copy, section by section.
- `02-feature-inventory.md` — source-verified capability inventory for ALTAI,
  IsanAgent, and Afterimage.
- `03-design-and-interaction-brief.md` — art direction, page rhythm, motion,
  typography, and interaction concepts.
- `04-competitive-benchmark.md` — current agentic-development website research
  and the useful pattern behind each reference.
- `05-interactive-app-replica-spec.md` — complete state, content, interaction,
  accessibility, and responsive specification for the app replica.
- `05-screenshot-catalog.md` — private visual references only; these PNGs must
  not be used as public website artwork.
- `06-source-map.md` — source revisions, primary references, and claim hygiene.
- `content-map.json` — structured navigation, hero, pillars, CTAs, and proof
  points for later implementation.
- `screenshots/` — original 2× PNG captures kept only for implementation and
  visual-QA reference.

## Voice

- Precise, assured, technical without sounding defensive.
- Outcome-led: say what ships, not merely what the agent can chat about.
- Concrete verbs: inspect, run, measure, review, recover, ship.
- No “magic,” “revolutionary,” “10×,” or vague productivity promises.
- Do not imitate Cursor, Kilo, Warp, Claude, or any other competitor’s copy.
- Treat open source, local-first execution, and model choice as operating
  principles rather than badge clutter.

## Visual premise

The current ALTAI product language is dark, compact, and operational. The
website should amplify that:

- solid black and graphite surfaces;
- warm white typography;
- ALTAI green used as an active-state signal, not as decoration;
- no gradients;
- minimal glow, only around live execution or verified-success states;
- restrained corner radius;
- a code-native, interactive replica of the real ALTAI interface as the primary
  product evidence;
- deterministic product-tour states that visitors can control;
- small monospace labels for state, scope, revision, and measured output.

## Recommended primary message

**Headline**

> Build software. Engineer models. One agentic workspace.

**Subhead**

> ALTAI is a local-first development environment where agents can understand
> your codebase, edit and verify code, research ML approaches, run experiments,
> generate training data, and carry the work all the way to a reviewable
> result.

**Primary CTA**

> Download ALTAI

**Secondary CTA**

> Explore the system

## Claim discipline

The feature inventory separates:

- features shipped in `altai-app`;
- capabilities embedded from the latest IsanAgent `main`;
- capabilities available in the latest Afterimage `main`;
- roadmap or experimental ideas that should not appear as shipped product
  claims.

The source snapshot was taken on 2026-07-29. See `06-source-map.md`.
