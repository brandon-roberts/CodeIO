# CodeIO Roadmap — Status Tracker

See `VISION.md` for the why. This file tracks where we are. Update it in every PR.

## Foundation (done / in progress)

- [x] Proto contracts for index, spotlight, context, depmap, AI, frontend, VM, meta
- [x] Rust services: codeio-index, codeio-spotlight, codeio-context, codeio-depmap (compile clean)
- [x] Build fixes: proto root path, OUT_DIR codegen, tokio-stream dep, borrow-after-move in symbol indexer (2026-07-06)
- [ ] Test suites for all four Rust crates (currently zero tests — top hygiene priority)
- [ ] Python AI layer: generate proto stubs, get `pytest` green
- [ ] CI pipeline (GitHub Actions: cargo build+test, ruff, mypy)

## M1 — Language Spec v0 (P3, P4)
- [x] Core syntax & semantics document (`docs/language-spec.md`) — v0 draft
- [x] Table declaration + relation syntax (spec §4)
- [x] Query expression syntax (spec §4)
- [x] `ai` primitive: call forms, typed outputs, caching (spec §5)
- [x] Inline metaprogramming forms (spec §6) — open questions tracked in §9

## M2 — Minimal Interpreter (P1, P6)
- [ ] `codeio-lang` crate: lexer, parser (consider folding Haskell layer into Rust — decision pending)
- [ ] Tree-walking interpreter; run `.cio` files end to end
- [ ] REPL

## M3 — Ollama Integration (P4)
- [ ] `proto/ai/inference_backend.proto`: backend-agnostic inference service
- [ ] Ollama client backend (Python layer first, Rust later)
- [ ] Content-hash response caching
- [ ] Context assembly via existing ContextWindowService for every call

## M4 — AI Line-by-Line Mode (P4)
- [ ] REPL mode where statements may be AI decisions/prompts/transforms
- [ ] Per-statement context budgeting

## M5 — Table Engine (P3)
- [ ] Storage + schema enforcement + relations
- [ ] Live queries; index integration with Spotlight
- [ ] Power-form rendering contract (proto) for the IDE

## M6–M7 — IDE Shell, 2D then 3D (P5)
- [ ] TypeScript LSP + web shell over live index
- [ ] WebGPU 3D dependency/topology explorer with live execution overlay

## M8 — Language Porting (P6)
- [ ] Tree-sitter grammar ingestion → CodeIO IR lifting
- [ ] Workspace language-detection scanner

## M9–M10 — P2P AI Landscape (P7)
- [ ] `proto/p2p/`: discovery, capability advertisement, job dispatch
- [ ] Remote-node mode: Android thin client → desktop Ollama node
- [ ] Sharded inference experiments with measured benchmarks

## M11 — Live VM (P1)
- [ ] Hot code swap, state inspection, live-edit round trip

## M12 — Semantic Self-Lift (P8)
- [ ] Lift CodeIO's own Rust layer into the CodeIO IR via the M8 porting pipeline
- [ ] Lift Python, Haskell, Lisp, and remaining host layers
- [ ] IDE explores CodeIO's own polyglot body in 2D/3D; AI answers questions about it via the index
- [ ] Extract/validate universal-language abstraction layers from the lifted corpus

## Decision log
- 2026-07-06: **DECIDED (Brandon):** Haskell frontend stays; the polyglot layer model is a feature, not a cost. Rust-consolidation proposal rejected. New pillar P8 (Polyglot Self-Composition) added. New layers (Kotlin/Swift/Go candidates) added only when a pillar demands them, each with protoc target + CI + health check in the same PR.
- 2026-07-06: P7 primary mode is remote-node dispatch (realistic perf); sharding is capacity play, not speed play.
