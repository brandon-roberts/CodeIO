# Language Services — the polyglot framework

**One IR, many adapters.** There is exactly one shared IR (`proto/ir/ir.proto`). Each host language
provides a `LanguageAdapter` service (`proto/lang/adapter.proto`) that lifts its source into that IR
and projects the IR back out. Adapters converge on the IR, never on each other — that is what keeps
the system universal instead of N×N translators.

## Fidelity ladder (honest capability, per adapter)
- **NONE** — slot reserved, nothing built.
- **DETECT_ONLY** — recognizes the language in a workspace scan (P6), cannot lift.
- **LIFT_ONLY** — source → IR: the code becomes indexable, queryable, visualizable, AI-addressable,
  and section-testable. No faithful projection back yet.
- **ROUND_TRIP** — source ↔ IR losslessly: full bidirectional blueprint/code sync.

Most languages will pass through DETECT → LIFT → ROUND_TRIP as we scale. LIFT_ONLY is already hugely
useful (import + understand + query any codebase); ROUND_TRIP is the summit (edit either side).

## Service slots (ready to fill)
Each is a language-service node implementing `LanguageAdapter`, tree-sitter-backed (M8):

| Language   | Role / why | Target fidelity | Status |
|------------|-----------|-----------------|--------|
| Python     | scripting, data, AI glue | ROUND_TRIP | NONE (slot) |
| JavaScript | web, ubiquitous | ROUND_TRIP | NONE (slot) |
| Java       | enterprise backends | LIFT_ONLY → ROUND_TRIP | NONE (slot) |
| C++        | systems, the VM layer | LIFT_ONLY | NONE (slot) |
| PHP        | legacy web, large install base | LIFT_ONLY → ROUND_TRIP | NONE (slot) |
| Lisp       | the metaprogramming layer (P2) | ROUND_TRIP | NONE (slot) |
| HTML       | presentation structure (P5) | ROUND_TRIP | NONE (slot) |
| CSS        | presentation style (P5) | ROUND_TRIP | NONE (slot) |
| Haskell    | the parse/typecheck frontend (P8) | LIFT_ONLY | NONE (slot) |
| Rust       | the services layer; self-lift target (P8/M12) | LIFT_ONLY | NONE (slot) |

## Build order
1. Shared IR contract ✅ (`proto/ir/ir.proto`)
2. Adapter contract ✅ (`proto/lang/adapter.proto`)
3. Materialize IR in the engine: interpreter emits `codeio.ir.Graph` instead of only tree-walking.
   (Pivotal — blueprint sync, section-addressing, and self-lift all require a real materialized IR.)
4. First adapter end-to-end: Python DETECT → LIFT (tree-sitter) → query lifted code.
5. Projection for round-trip languages; then fill remaining slots.

## Why this order flags nothing false
No adapter is claimed done. The IR is defined before adapters target it. Fidelity is declared per
adapter so the registry never overstates what a language can do. This is framework-laid-and-ready,
not five fake IRs.
