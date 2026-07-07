# CodeIO Feature Status

<!-- GENERATED FILE — do not edit. Edit features.toml and run tools/scripts/gen_docs.py -->

_Regenerated 2026-07-07 — 5 live · 8 building · 21 planned_

Legend: ✅ LIVE = working end-to-end · 🚧 BUILDING = code exists, not yet proven · 📋 PLANNED = theory/design only

| Status | Feature | Pillar | Entry point | Description |
|--------|---------|--------|-------------|-------------|
| ✅ LIVE | **codeio CLI (main entry point)** | P1 | `services/crates/codeio-cli` | Single entry point: start services, check status, list features. `cargo run -p codeio-cli -- --help` |
| ✅ LIVE | **codeio doctor (system analyzer, go/no-go)** | P1 | `services/crates/codeio-cli` | Scans toolchains (Rust/protoc/python/git/node/ghc/g++), AI backends (Ollama), and CodeIO services; per-OS install hints; exits nonzero on NO-GO for CI use. |
| ✅ LIVE | **Minimal interpreter (M2)** | P1/P6 | `services/crates/codeio-lang` | Lexer, Pratt parser, tree-walking interpreter with closures; codeio run + codeio repl; 9-test suite green. Tables/ai/meta/bridges land next. |
| ✅ LIVE | **Proto3 cross-language contracts** | P1/P8 | `proto/` | All cross-layer APIs defined in Protocol Buffers; source of truth for every service. |
| ✅ LIVE | **Evolution architecture model (tiers + coherence plugins)** | P1/P8 | `docs/architecture-model.md` | L0/L1/L2 normalization; no lateral coupling; parts bound only by higher-tier coherence plugins. |
| 🚧 BUILDING | **Dependency map service** | P1 | `services/crates/codeio-depmap` | Import/dependency graph resolver. |
| 🚧 BUILDING | **Language spec v0 (M1)** | P2/P3/P4 | `docs/language-spec.md` | Syntax + semantics: table declarations, query expressions, ai primitive, inline meta forms. |
| 🚧 BUILDING | **Workspace indexer service** | P3 | `services/crates/codeio-index` | Scans workspaces into content-addressed IndexEntry chunks with symbol records. Compiles; needs tests + end-to-end exercise. |
| 🚧 BUILDING | **Spotlight hybrid search** | P3/P4 | `services/crates/codeio-spotlight` | Trigram inverted index + semantic tier with RRF fusion. Compiles; needs tests + end-to-end exercise. |
| 🚧 BUILDING | **Context window engine** | P4 | `services/crates/codeio-context` | Token-budgeted, relevance-ranked context assembly (greedy knapsack). The small-context AI discipline lives here. |
| 🚧 BUILDING | **Python AI layer** | P4 | `ai/codeio` | LLM client, tool dispatcher, context window manager. Needs proto stubs generated + tests. |
| 🚧 BUILDING | **P7 mesh contract (node model, capacity, transport, block streaming)** | P7 | `proto/p2p/mesh.proto` | Proto for local-first resource mesh: measured Capacity, LAN/Bluetooth/Direct/Internet transport preference, streamed WorkBlocks, aggregate MeshSummary ('LN as one big node'). |
| 🚧 BUILDING | **Architecture Authority (conformance refs + escalation)** | P9 | `VISION.md` | Developer-as-author doctrine: AI in conformance/escalation modes only; IR nodes cite the authorizing artifact; practiced now, enforced mechanically once IR lands. |
| 📋 PLANNED | **C++ VM with hot swap (M11)** | P1 | `proto/vm/` | Bytecode interpreter with live code swap + state inspection. |
| 📋 PLANNED | **codeio serve (application server mode)** | P1/P7 | `—` | Long-running server: hosts the UPL engine, serves the IDE web shell and APIs over LAN/Tailscale; the deployment form for phones and teams. |
| 📋 PLANNED | **cio pkg (UPL package manager)** | P1/P8 | `—` | Internal package manager for .cio libraries and bridges: content-addressed packages, version resolution, lockfiles; the registry protocol doubles as the P7 mesh distribution channel. |
| 📋 PLANNED | **Lisp macro/DSL layer** | P2/P8 | `proto/meta/` | Macro expansion, DSL framework, homoiconic quasi-quote services. |
| 📋 PLANNED | **Table-native data engine (M5)** | P3 | `—` | Typed schemas, relations, live queries, power-forms; wired into the Spotlight index. |
| 📋 PLANNED | **Ollama inference backend (M3)** | P4 | `proto/ai/inference_backend.proto` | Backend-agnostic inference service with Ollama as first-class local backend; content-hash caching. |
| 📋 PLANNED | **AI line-by-line mode (M4)** | P4 | `—` | REPL where statements may be AI decisions/prompts/transforms, each with exact-fit context. |
| 📋 PLANNED | **IDE shell 2D (M6)** | P5 | `—` | Node/table/text views over the live index. TypeScript + WebGPU foundation. |
| 📋 PLANNED | **3D exploration (M7)** | P5 | `—` | Navigable 3D dependency/topology space with live execution overlay. |
| 📋 PLANNED | **GUI layers over CLI and language** | P5 | `docs/architecture-model.md` | GUI shells as L2 plugins over the same proto contracts as the CLI; .cio constructs render as live surfaces (power-forms, query views, ai cards). |
| 📋 PLANNED | **App store + plugin/extension manager** | P5/P8 | `—` | Ecosystem layer over cio pkg: apps, IDE plugins, language extensions, and UPL flavors, each an L2 coherence plugin with conformance references; MAGIC-curated store. |
| 📋 PLANNED | **Language porting via tree-sitter (M8)** | P6 | `—` | Detect languages in a system and lift them into the CodeIO IR. |
| 📋 PLANNED | **magic.coinos bridge (finance engine)** | P6 | `https://github.com/brandon-roberts/CoinOS/tree/main/proto/coinos` | CodeIO consumes CoinOS (MAGIC finance engine) via bridge: BrokerBridge multi-venue trading, DecisionLedger, Treasury self-billing with budget guards. |
| 📋 PLANNED | **Bridge system: protocol imports + sync check** | P6/P8 | `docs/language-spec.md#7` | Host-language capabilities as generated, content-addressed bridge libraries with scoped imports, exec-time generation, staleness detection, and codeio bridge rebuild. |
| 📋 PLANNED | **Universal language coverage (Java/PHP/Python/C++/JS/HTML/CSS/...)** | P6/P8 | `docs/language-spec.md#7` | Every mainstream language reachable two ways: tree-sitter lifting into the IR (read/query/edit) and bridges for live capability calls; HTML/CSS lift as declarative IR for the presentation layer. |
| 📋 PLANNED | **P2P AI landscape (M9-M10)** | P7 | `—` | Device mesh: discovery, capability ads, remote-node Ollama dispatch; sharded inference experiments. |
| 📋 PLANNED | **Remote-node inference dispatch (the primary P7 win)** | P7 | `docs/mesh-design.md` | Thin client streams a job to the strongest local node (desktop Ollama) and streams results back — cheap phone, workstation quality, one LAN hop. |
| 📋 PLANNED | **codeio doctor --bench (measured capacity)** | P7 | `services/crates/codeio-cli` | Short local benchmark producing real tokens/sec and link throughput into Capacity; aggregate mesh numbers computed from measurements, never spec sheets. |
| 📋 PLANNED | **Haskell parse/typecheck services** | P8 | `proto/frontend/` | Lexer, parser, Hindley-Milner type inference as gRPC services. |
| 📋 PLANNED | **Semantic self-lift (M12)** | P8 | `—` | CodeIO's own polyglot source lifted into its IR; the system explores and reasons about itself. |
| 📋 PLANNED | **Physical canvas (sketch->IR, spatial coords, glyph notation)** | P9 | `VISION.md` | Whiteboard/paper ingestion via vision models with human confirmation; IR nodes carry spatial coordinates; glyph notation as IR projection; continuous canvas-watch (mounted/phone camera, change-detected regions, proposed IR diffs). |

See `VISION.md` for the pillars and `ROADMAP.md` for milestone tracking.
