# CodeIO Visual Workbench — self-hosting blueprint designer

Status: 📋 planned (summit). Authored intent per Architecture Authority. Foundations below it are
being built; the workbench is the destination they converge on. No part of this is faked as done.

## What it is
A visual, blueprint-first software engineering tool where you construct systems by clicking/zooming/
spanning a live projection of the IR — not by typing all code. The designer can extend the very
language it is built in: new rules, syntax, and capabilities are authored as blueprints (declarative
selector/mapping rules the engine already consumes), closing the self-hosting loop (P8).

## The UI stack (decided)
The **designer surface** uses HTML/CSS/JS — because it is a developer tool, not an app built in CodeIO.
This cleanly separates concerns:
- Designer chrome (the IDE itself) = web tech, runs on our infrastructure / `codeio serve`.
- Systems built in the designer = pure IR, run by any engine (Rust/Python/…), streamed anywhere.
No native-renderer dependency is required to start; the native/2D-3D renderer (P5) is a later climb
for the *rendered systems*, not the designer chrome.

## Semantic zoom (the core interaction)
Every zoom level is a projection of the SAME content-addressed IR at a different tier:
1. **Blueprint** (L2) — modules and their connections; coherence view.
2. **UPL view** — the CodeIO source projection of a region.
3. **Lower-language constructs** (L1) — how nodes route to Rust/Python/JS/… (produced by codeio-router).
4. **Source & libraries** (L0) — emitted code and lifted library surfaces.
Click a blueprint block → zoom to its lower constructs. The router's bidirectional index IS the link
between zoom levels; the trace log IS the real-time stack trace; the ContextWindow engine IS the
context buffer at the current focus.

## Why the foundations make it inevitable (not magic)
- IR is materialized + content-addressed (codeio-ir, live) → blueprints are IR projections.
- Router matches+routes+indexes, stack-traceable (codeio-router, live) → zoom links + traces.
- Selector engine authors syntax/capabilities as rules → visual language extension.
- Tier normalization (L0/L1/L2) → each zoom level is a tier view; no coupling to untangle.

## Honest dependency ladder
1. codeio-router (selector/router/index) ✅ built
2. IR-driven evaluator (execute from graph) — in progress
3. `codeio serve` + web chrome — planned
4. Projection (IR → source per stack) + first adapters — planned
5. Interactive blueprint editor + semantic zoom — the summit
