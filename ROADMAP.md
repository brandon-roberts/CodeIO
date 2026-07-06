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
- [ ] Core syntax & semantics document (`docs/language-spec.md`)
- [ ] Table declaration + relation syntax
- [ ] Query expression syntax (statically checked against schemas)
- [ ] `ai` primitive: call forms, typed/schema-constrained outputs, caching semantics
- [ ] Inline metaprogramming forms (quote/unquote at IR level, self-query API)

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

## Decision log
- 2026-07-06: Proposed folding parsing/typechecking into Rust instead of Haskell for velocity and deployability. **Pending Brandon's sign-off.**
- 2026-07-06: P7 primary mode is remote-node dispatch (realistic perf); sharding is capacity play, not speed play.
