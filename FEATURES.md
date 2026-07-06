# CodeIO Feature Status

<!-- GENERATED FILE — do not edit. Edit features.toml and run tools/scripts/gen_docs.py -->

_Regenerated 2026-07-06 — 3 live · 6 building · 14 planned_

Legend: ✅ LIVE = working end-to-end · 🚧 BUILDING = code exists, not yet proven · 📋 PLANNED = theory/design only

| Status | Feature | Pillar | Entry point | Description |
|--------|---------|--------|-------------|-------------|
| ✅ LIVE | **codeio CLI (main entry point)** | P1 | `services/crates/codeio-cli` | Single entry point: start services, check status, list features. `cargo run -p codeio-cli -- --help` |
| ✅ LIVE | **Proto3 cross-language contracts** | P1/P8 | `proto/` | All cross-layer APIs defined in Protocol Buffers; source of truth for every service. |
| ✅ LIVE | **Evolution architecture model (tiers + coherence plugins)** | P1/P8 | `docs/architecture-model.md` | L0/L1/L2 normalization; no lateral coupling; parts bound only by higher-tier coherence plugins. |
| 🚧 BUILDING | **Dependency map service** | P1 | `services/crates/codeio-depmap` | Import/dependency graph resolver. |
| 🚧 BUILDING | **Language spec v0 (M1)** | P2/P3/P4 | `docs/language-spec.md` | Syntax + semantics: table declarations, query expressions, ai primitive, inline meta forms. |
| 🚧 BUILDING | **Workspace indexer service** | P3 | `services/crates/codeio-index` | Scans workspaces into content-addressed IndexEntry chunks with symbol records. Compiles; needs tests + end-to-end exercise. |
| 🚧 BUILDING | **Spotlight hybrid search** | P3/P4 | `services/crates/codeio-spotlight` | Trigram inverted index + semantic tier with RRF fusion. Compiles; needs tests + end-to-end exercise. |
| 🚧 BUILDING | **Context window engine** | P4 | `services/crates/codeio-context` | Token-budgeted, relevance-ranked context assembly (greedy knapsack). The small-context AI discipline lives here. |
| 🚧 BUILDING | **Python AI layer** | P4 | `ai/codeio` | LLM client, tool dispatcher, context window manager. Needs proto stubs generated + tests. |
| 📋 PLANNED | **C++ VM with hot swap (M11)** | P1 | `proto/vm/` | Bytecode interpreter with live code swap + state inspection. |
| 📋 PLANNED | **Minimal interpreter (M2)** | P1/P6 | `services/crates/codeio-lang` | Lexer, parser, tree-walking interpreter; run .cio files. |
| 📋 PLANNED | **Lisp macro/DSL layer** | P2/P8 | `proto/meta/` | Macro expansion, DSL framework, homoiconic quasi-quote services. |
| 📋 PLANNED | **Table-native data engine (M5)** | P3 | `—` | Typed schemas, relations, live queries, power-forms; wired into the Spotlight index. |
| 📋 PLANNED | **Ollama inference backend (M3)** | P4 | `proto/ai/inference_backend.proto` | Backend-agnostic inference service with Ollama as first-class local backend; content-hash caching. |
| 📋 PLANNED | **AI line-by-line mode (M4)** | P4 | `—` | REPL where statements may be AI decisions/prompts/transforms, each with exact-fit context. |
| 📋 PLANNED | **IDE shell 2D (M6)** | P5 | `—` | Node/table/text views over the live index. TypeScript + WebGPU foundation. |
| 📋 PLANNED | **3D exploration (M7)** | P5 | `—` | Navigable 3D dependency/topology space with live execution overlay. |
| 📋 PLANNED | **GUI layers over CLI and language** | P5 | `docs/architecture-model.md` | GUI shells as L2 plugins over the same proto contracts as the CLI; .cio constructs render as live surfaces (power-forms, query views, ai cards). |
| 📋 PLANNED | **Language porting via tree-sitter (M8)** | P6 | `—` | Detect languages in a system and lift them into the CodeIO IR. |
| 📋 PLANNED | **Bridge system: protocol imports + sync check** | P6/P8 | `docs/language-spec.md#7` | Host-language capabilities as generated, content-addressed bridge libraries with scoped imports, exec-time generation, staleness detection, and codeio bridge rebuild. |
| 📋 PLANNED | **P2P AI landscape (M9-M10)** | P7 | `—` | Device mesh: discovery, capability ads, remote-node Ollama dispatch; sharded inference experiments. |
| 📋 PLANNED | **Haskell parse/typecheck services** | P8 | `proto/frontend/` | Lexer, parser, Hindley-Milner type inference as gRPC services. |
| 📋 PLANNED | **Semantic self-lift (M12)** | P8 | `—` | CodeIO's own polyglot source lifted into its IR; the system explores and reasons about itself. |

See `VISION.md` for the pillars and `ROADMAP.md` for milestone tracking.
