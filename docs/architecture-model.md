# CodeIO Architecture Model — Built to Evolve

Status: canonical, alongside VISION.md. Every module, service, and PR must state its tier.

## The rule that makes evolution possible

**No two parts may couple directly. Parts are made coherent only by a plugin one tier above them.**

A "part" is any unit: a Rust crate, a proto service, a language feature, a GUI panel, a bridge.
Parts expose contracts (protos) and know nothing about their peers. A *coherence plugin* at the
tier above binds parts together into a behavior. Replace, upgrade, or redesign any part and only
its contract matters; delete a coherence plugin and the parts it bound remain intact and reusable.

Corollaries:
- Lateral imports between peer parts are forbidden. If part A needs part B, that need is itself
  a contract, satisfied by a plugin above.
- Every part is independently buildable, testable, and versionable.
- Contracts are content-addressed; a part declares which contract *hashes* it satisfies, so
  staleness is detectable mechanically (see Bridge Sync in the language spec §7.3).

## The three tiers — hard criteria

### L0 — Low level (mechanism)
*Criteria: owns memory/compute/persistence primitives; no policy; no knowledge of the language,
IDE, or AI; deterministic; zero dependencies on L1/L2.*
- C++ VM, memory, bytecode. Rust storage/index primitives (trigram index, content store).
- Host-language standard capabilities exposed as raw proto services.
- May depend on: nothing above L0.

### L1 — Middleware (capability)
*Criteria: composes L0 mechanisms into named capabilities with policy; stateless where possible;
speaks only proto; no rendering; no user interaction.*
- Spotlight search, ContextWindow assembly, DepMap, table engine, inference backends
  (Ollama/Anthropic/mesh), bridge generators, orchestrator.
- Coherence plugins that bind L0 parts live here.
- May depend on: L0 contracts only.

### L2 — High level (experience & expression)
*Criteria: user-facing meaning — the language surface, REPL, CLI, GUI, IDE 2D/3D; policy-rich;
never touches L0 directly.*
- The CodeIO language + interpreter, `codeio` CLI, GUI shells, IDE, AI line-by-line mode.
- Coherence plugins that bind L1 capabilities into experiences live here.
- May depend on: L1 contracts only.

**Normalization check (apply to every new part):** state (1) its tier, (2) the contracts it
exposes, (3) the contracts it consumes — which must be exactly one tier below. Anything that
can't be stated that way is two parts that haven't been separated yet.

## GUI layering (P5, made concrete)

GUIs are L2 coherence plugins over the same contracts the CLI uses — never a separate system:

1. **GUI-over-CLI:** every `codeio` subcommand is also a proto service call; a GUI shell
   (desktop/web) renders the same calls as panels. CLI and GUI cannot drift because they are
   two skins over one contract.
2. **GUI-over-language:** `.cio` constructs render as live surfaces — a `table` renders as a
   power-form, a `live` query as an auto-updating view, an `ai` call as an inspectable card,
   IR as node graph (2D) or topology (3D). Declared in the language, rendered by L2 plugins.

## Evolution playbook

- **Upgrade a part:** ship new part satisfying same contract hash lineage; plugins unaffected.
- **Redesign a behavior:** replace the coherence plugin; parts unaffected.
- **Reuse a part:** point a new plugin at its contract; nothing to untangle.
- **Deprecate:** contracts carry version + hash; the sync checker flags every consumer.
