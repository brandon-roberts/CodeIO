# CodeIO Vision

**The canonical statement of what CodeIO is becoming. Every design decision must trace back to a pillar in this document. If work doesn't serve a pillar, it doesn't belong in the repo.**

---

## One-sentence definition

CodeIO is a frontier, enterprise-grade IDE and universal programming language in which code, data, and AI are the same substance: live, inspectable, queryable, and editable at every layer, on any device, with AI context assembled precisely rather than dumped.

---

## The Pillars

### P1 — Transparent Layered Engineering
Every layer of the system — source text, AST, typed IR, bytecode, runtime state — is a first-class, addressable, *live* artifact. Editing any layer propagates to the others in real time. Nothing is a black box: a developer (or the AI) can open the pipeline at any stage, watch data flow through it, and modify it while it runs.

- Contracts between layers are Protocol Buffers (`proto/`) — the single source of truth.
- The VM supports hot code swap and state inspection as core features, not add-ons.
- "Live code in and out": the running program is an editable object, not a compiled artifact.

### P2 — Inline Metaprogramming
Programs that scaffold, analyze, and debug *themselves* are written in the same language, inline, not in a separate macro dialect. The language is homoiconic at the IR level: code is data that the program can query, generate, and rewrite at runtime.

- Macro expansion, DSL definition, and self-analysis are language primitives.
- A program can ask "what calls this function?", "generate a test for this", "why did this value change?" from within itself.

### P3 — Data as Tables (the Xcode / Android Studio standard)
Structured data is table-native in the language: typed schemas, live queries, key relations, and indexes are language constructs, not library bolt-ons.

- Declare a table like you declare a type. Relations between keys are declared and enforced.
- Queries are first-class expressions, statically checked against schemas.
- The same indexing engine that powers Spotlight search powers user data: trigram + semantic hybrid, exposed both to the developer's code and to the AI's context assembler.
- The IDE renders tables as tables — sortable, filterable, editable power-forms — never as JSON blobs.

### P4 — AI as a Language Primitive (Ollama-native)
AI inference is a callable primitive of the language, with Ollama as a first-class local backend alongside cloud APIs. There is an **AI line-by-line mode**: an interactive style of programming in which each statement may be an AI decision, prompt, or transformation.

- The key discipline: **minimal context, maximal correctness.** Every AI call receives a context window assembled by the ContextWindowService — token-budgeted, relevance-ranked, exact-fit for the task. Never "dump the file and pray."
- AI calls compose like functions: typed inputs, typed (schema-constrained) outputs, deterministic caching by content hash so repeated identical calls cost nothing.
- The data-table layer (P3) is the AI's lookup memory: instead of stuffing facts into the prompt, the AI queries tables at inference time.

### P5 — Frontier IDE: 2D/3D Rendered, Fully Explorable
This is not another AI web-app IDE with a chat sidebar. The IDE renders the codebase as an explorable space:

- **3D mode:** the dependency graph, service topology, and data-flow are navigable 3D structures (GPU-rendered; WebGPU/wgpu). Fly through the architecture; watch live execution light up the paths it takes.
- **2D mode:** power-forms, table editors, node-graph editing of the IR, classic text editing — all views over the same underlying content-addressed model.
- Every visual element is *live*: click a node in the 3D graph and you are editing the real running artifact (P1).
- Enterprise-grade means: multi-workspace, RBAC-ready, auditable, performant on large monorepos — not a toy demo.

### P6 — Universal Language + Language Porting
The language is universal in two directions:

- **Outbound:** one CodeIO definition compiles/transpiles to multiple targets via the language-agnostic IR.
- **Inbound:** CodeIO can *notice* languages present in a system (scan the workspace, detect grammars, toolchains, project files) and **port them in** — generating parser adapters that lift foreign code into the CodeIO IR so it becomes indexable, queryable, visualizable, and AI-addressable like native code. Tree-sitter grammars are the on-ramp: any language with a grammar can be lifted.

### P7 — P2P AI Landscape
AI compute is a network resource. Devices on the user's landscape (phones, laptops, desktops, servers) form a peer-to-peer mesh that serves inference cooperatively:

- **Remote-node mode (primary):** a lightweight client on a cheap Android device dispatches inference to the strongest node(s) on the mesh (e.g., a Mac Pro running Ollama) and streams results back. The phone gets big-model quality because the heavy lifting happens elsewhere. This is the realistic path to "high-tier Ollama on a cheap phone."
- **Sharded mode (secondary):** models split across multiple peers (layer-parallel, in the style of exo/Petals/llama.cpp RPC) so the mesh can serve models no single node fits. Honest engineering note: sharding over consumer networks adds latency per token; it buys *capacity* (bigger models), not raw speed. Design accordingly — use it when the model doesn't fit anywhere, use remote-node mode when it does.
- **Central-coordinator mode:** one orchestrating model routes sub-tasks to specialist models across the mesh, aggregating results.
- Discovery (mDNS/DHT), capability advertisement (VRAM, model list, tokens/sec), authenticated transport, and job scheduling are all proto-defined services in the existing gRPC fabric.

### P8 — Polyglot Self-Composition
CodeIO is itself composed of many host languages, deliberately, and unifies them with its own machinery. The system is the first proof of the universal-language claim.

Two unification planes:

- **Runtime plane (exists today):** every layer speaks through proto-defined gRPC contracts. Adding a language layer = adding a protoc target + a service implementation. No layer knows or cares what language its peers are written in.
- **Semantic plane (the ambition):** every host layer's *own source code* is lifted into the CodeIO IR via the porting pipeline (P6 turned inward). Once lifted, the Rust indexer, Haskell parser, Lisp macro engine, and every other layer become indexable, queryable, 3D-explorable, and AI-addressable through the same machinery they implement. The abstraction layers of the universal language are then not designed on a whiteboard — they are *extracted* from a working multi-language system that already exercises them.

Layer assignments follow one rule: each language owns the layer it is genuinely best at. The Haskell frontend stays (types and parsing are its home turf). New layers are added when a pillar demands a language's strengths — e.g., Kotlin for the Android thin client (P7), Swift for iOS, Go for the P2P mesh networking — never for variety's sake.

Cost discipline: every layer added must ship with its protoc target, CI job, and health check in the same PR. A layer without CI is a liability, not a paradigm.

### P9 — Physical Canvas, Developer Authorship, Dual Execution
**Physical canvas:** the software canvas extends into the real world. Whiteboard/paper sketches are photographed, vision-extracted (boxes, arrows, algorithm glyphs, handwriting), and proposed as IR — always human-confirmed, never silently absorbed. Every IR node may carry spatial coordinates (page, board, or 3D scene position), making location queryable and driving the P5 world. A hand-drawable glyph notation (map/fold/branch/recur symbols) is a first-class projection of the IR, equal to text syntax — handwriting and design skills return as programming input.

**Architecture Authority (developer-as-author):** all architectural decisions live in developer-authored artifacts — VISION.md, the language spec, the architecture model, protos, features.toml, the decision log. AI operates in two modes only: *conformance* (implement exactly what the artifacts define; silence in the artifacts is a gap, not a license) and *escalation* (surface undecided questions as decision requests — never decide silently). Severity line: anything touching contracts, tiers, data models, or public interfaces escalates; below that is implementation detail in the developer's established style. Every AI-produced IR node carries a conformance reference to the artifact that authorized it; code that cannot cite its authority is flagged. The developer is the author; tool-provenance receipts are kept internally so conformance is provable, not merely claimed.

**Dual execution:** the UPL is a real engine, not a transpiler. `codeio run` executes CodeIO directly (tree-walker today → bytecode VM → optional JIT); `codeio build --target <lang>` lowers to native executables when deployment or raw speed demands it. Lowering is a choice, never a dependency. Imported foreign code lives as IR; edits at any abstraction level are IR mutations absorbed by the living model and re-projected to lower languages — never static one-way translation.

---

## Non-negotiable engineering disciplines

1. **Proto-first.** No cross-layer API exists until its `.proto` exists.
2. **Content-addressed everything.** Code chunks, table rows, AI calls, IR nodes — all identified by content hash. This is what makes caching, dedup, P2P distribution, and live-sync tractable.
3. **Small context is a feature.** Any PR that grows AI prompt sizes without a relevance-ranking justification is wrong by default.
4. **Tests land with features.** The repo currently has zero tests; that ends now.
5. **Honest performance claims.** We measure; we do not assume. Especially for P7.

---

## How the pillars interlock

P3 (tables) + P2 (self-analysis) feed P4 (exact-fit AI context). P4 + P7 make AI ubiquitous and cheap. P1 (live layers) + P5 (rendered exploration) make the whole thing visible and touchable. P6 makes it apply to every codebase, not just CodeIO-native ones. P8 turns P6 inward: CodeIO's own polyglot body is the permanent test bed for the universal language's abstraction layers. The content-addressed index is the spine connecting all of it.

---

## Roadmap (living — reorder as reality dictates)

| Milestone | Pillar(s) | Deliverable |
|---|---|---|
| M1 | P3, P4 | Language spec v0: syntax, table declarations, `ai` primitive semantics, query expressions |
| M2 | P1, P6 | Minimal Rust parser + tree-walking interpreter; runnable `.cio` programs |
| M3 | P4 | Ollama backend behind ContextWindowService; typed `ai` calls with schema-constrained output + content-hash caching |
| M4 | P4 | AI line-by-line REPL mode |
| M5 | P3 | Table engine: schemas, relations, live queries wired into the Spotlight/Context index |
| M6 | P5 | IDE shell: 2D node/table views over the live index (TypeScript + WebGPU foundation) |
| M7 | P5 | 3D codebase/dependency exploration with live execution overlay |
| M8 | P6 | Tree-sitter-based language porting: lift foreign code into the IR |
| M9 | P7 | P2P mesh v0: discovery + remote-node Ollama dispatch (Android thin client → desktop node) |
| M10 | P7 | Sharded/coordinated inference experiments, measured honestly |
| M11 | P1 | Hot code swap + live state inspection in the VM |

Status tracking lives in `ROADMAP.md` checkboxes; this file states *why*, that file states *where we are*.
