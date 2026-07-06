# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Vision Alignment

**Read `VISION.md` first.** It is the canonical statement of the seven pillars (P1–P7). Every change must serve a pillar. Track progress in `ROADMAP.md` and update it in every PR.

## What This Project Is

CodeIO is a polyglot meta-language and AI-integrated codebase management system. It consists of:
1. A language runtime built across 8 host languages, each responsible for the layer it is best suited for
2. A suite of gRPC services that expose codebase intelligence (indexing, search, context assembly)
3. An AI integration layer that gives LLMs structured access to those services
4. A context window engine that solves the "large codebase" problem by returning exactly what the AI needs, ranked by relevance, within a token budget

## Language Layer Assignments

Each language owns one architectural layer. Never implement a concern in the wrong layer.

| Layer | Language | Responsibility |
|-------|----------|---------------|
| 0 | Proto3 | Universal contracts — all cross-language data structures and service interfaces |
| 1 | C++ | Core VM/runtime, bytecode interpreter, GC, memory allocator, native FFI |
| 2 | Rust | Workspace indexer, spotlight search, context window engine, dependency map service |
| 3 | Haskell | Lexer, parser, AST, Hindley-Milner type system, type inference server |
| 4 | Lisp (Clojure/Racket) | Macro expansion, DSL framework, homoiconicity, quasi-quote |
| 5 | Python | AI/ML integration, LLM client, tool dispatcher, context assembly client |
| 6 | Java | Service orchestrator, process manager, service registry, workflow engine |
| 7 | TypeScript | Language Server Protocol server, IDE integration, web UI |
| 8 | PHP | Web gateway, plugin system, developer portal |

## Repository Layout

```
proto/          # Layer 0: All .proto files — the source of truth for every cross-language API
  core/         #   FileRef, Span, Position, Language, SeverityLevel
  index/        #   WorkspaceScanService, ContextIndexService, DependencyMapService
  ai/           #   AIService, SpotlightService, ContextWindowService, shared types
  frontend/     #   ParseService, TypeCheckService, AST node types
  vm/           #   VMControlService, ExecutionService
  meta/         #   MacroService, DSLService
services/       # Layer 2: Rust workspace (Cargo workspace with 5 crates)
  crates/
    codeio-common/    # Generated proto bindings, re-exported for all other crates
    codeio-index/     # WorkspaceScanService + ContextIndexService gRPC servers
    codeio-spotlight/ # SpotlightService — trigram index + semantic search
    codeio-context/   # ContextWindowService — token-budget context assembly
    codeio-depmap/    # DependencyMapService — import graph resolver
ai/             # Layer 5: Python AI integration
  codeio/
    services/         # gRPC client wrappers (spotlight.py, client_pool.py)
    context/          # WindowManager — assembles ContextWindow via Rust service
    llm/              # client.py (Anthropic SDK), tool_dispatcher.py
tools/
  protogen/     # generate.sh — runs protoc for all 8 language targets
  build/        # Makefile, docker-compose.yml
  scripts/      # health-check.sh
config/
  codeio.toml   # Master config — service ports, token budgets, index settings
```

## Build Commands

```bash
# Proto codegen (must run first before building any service)
./tools/protogen/generate.sh all         # All 8 languages
./tools/protogen/generate.sh python      # Python only (fastest for AI layer work)

# Rust services
cd services && cargo build               # Debug build
cd services && cargo build --release     # Release build
cd services && cargo test                # All Rust tests

# Run individual Rust services locally
cd services && RUST_LOG=info cargo run -p codeio-index
cd services && RUST_LOG=info cargo run -p codeio-spotlight
cd services && RUST_LOG=info cargo run -p codeio-context
cd services && RUST_LOG=info cargo run -p codeio-depmap

# Python AI layer
cd ai && pip install -e ".[dev]"         # Install with dev extras
cd ai && pytest tests/ -v                # Tests
cd ai && ruff check codeio/             # Lint
cd ai && mypy codeio/                   # Type check

# Full local stack via Docker
make docker-up                           # Start all services
make docker-logs                         # Stream logs
make docker-down                         # Stop
make health                              # Check all services respond

# Top-level shortcuts (from repo root)
make proto                               # Regenerate all protos
make rust                                # Build all Rust crates
make python                              # Install Python deps
make all                                 # proto + rust + python
```

## IPC Architecture

All cross-language communication uses **gRPC over Unix domain sockets** (local) or TCP (cross-host). No in-process FFI except C++ hosting the VM.

Service ports (configured in `config/codeio.toml`, overridable via env vars):
```
VM         :50050  (C++)
Index      :50052  (Rust) — CODEIO_INDEX_ADDR
Spotlight  :50053  (Rust) — CODEIO_SPOTLIGHT_ADDR
Context    :50054  (Rust) — CODEIO_CONTEXT_ADDR
DepMap     :50055  (Rust) — CODEIO_DEPMAP_ADDR
Parse      :50060  (Haskell)
TypeCheck  :50061  (Haskell)
Meta       :50070  (Lisp)
AI         :50080  (Python)
Orchestrator :50090 (Java)
```

Large context windows (>4MB) use a **mmap protocol**: the Rust context service writes to `/tmp/codeio/ctx_{id}.bin` and returns only the file path + SHA256 checksum. Python verifies the checksum before deserializing.

## Core Data Structures

**`IndexEntry`** (`proto/index/context_index.proto`) — the atomic unit of the codebase index. One entry per parse-meaningful chunk (function, class, import block, module header). Contains: `file_ref`, `span`, `chunk_kind`, `symbol_record`, `raw_content`, `tokens`, `importance_score`.

**`ContextWindow`** (`proto/ai/context_window.proto`) — the assembled context for one AI call. Contains ranked `ContextSlice[]`, each with relevance score and include reason. Token-budget aware: greedy knapsack fills the budget with highest-relevance entries first.

**`SpotlightQuery/Results`** (`proto/ai/spotlight.proto`) — two-tier search. Tier 1: trigram inverted index (sub-millisecond). Tier 2: vector embedding (semantic, milliseconds). HYBRID mode fuses both via Reciprocal Rank Fusion.

**`FocusPoint`** (`proto/ai/types.proto`) — what the AI's attention is on: a cursor position, a symbol, or a query string. The context assembler uses this to rank relevance.

## AI Tool Library

The Python `ToolDispatcher` (`ai/codeio/llm/tool_dispatcher.py`) maps LLM tool calls to services:

| Tool name | Routes to | Purpose |
|-----------|-----------|---------|
| `spotlight` | Rust SpotlightService | Find symbols, functions, strings across the codebase |
| `get_context` | Rust ContextWindowService | Assemble ranked code context within token budget |
| `read_file` | Filesystem | Read a specific file or line range |
| `dependency_map` | Rust DependencyMapService | Import/dependency graph for a file |

The `CodeIOClient` (`ai/codeio/llm/client.py`) handles multi-round tool use automatically. The LLM calls tools until it has enough context, then generates a final response.

## Adding a New Proto Service

1. Write the `.proto` file in the appropriate `proto/` subdirectory
2. Run `./tools/protogen/generate.sh all` (or the specific language)
3. Implement the gRPC server in the appropriate language layer
4. Add the service to `config/codeio.toml` with its port
5. Update `ai/codeio/llm/tool_dispatcher.py` if the AI should be able to call it

## Build Order (for new contributors)

Phase 0 is done: all `.proto` files exist. When implementing new layers, follow this sequence:
- **Phase 1**: C++ VM (`runtime/`) — bytecode interpreter, no external dependencies
- **Phase 2**: Rust services (`services/`) — done; indexer, spotlight, context, depmap
- **Phase 3**: Haskell frontend (`frontend/`) — parser, type checker
- **Phase 4**: Lisp meta layer (`meta/`) — macro expander, DSL framework  
- **Phase 5**: Python AI (`ai/`) — partially done; needs proto stubs generated first
- **Phase 6**: Java orchestrator (`orchestrator/`) — process manager, service registry
- **Phase 7**: TypeScript LSP (`lsp/`) — language server
- **Phase 8**: PHP gateway (`gateway/`) — web API, plugin system

## Branch Strategy

Active development branch: `claude/environment-optimization-pbll5x`

```bash
git push -u origin claude/environment-optimization-pbll5x
```

## GitHub Integration

Use `mcp__github__*` MCP tools for all GitHub interactions. The `gh` CLI is not available.
Scoped to repository: `brandon-roberts/CodeIO`

## Environment Notes

- Running in a remote ephemeral container — commit and push before the session ends
- `ANTHROPIC_API_KEY` must be set in the environment or `.env` for the Python AI layer
- Rust build requires protoc to be installed for the `build.rs` codegen step
