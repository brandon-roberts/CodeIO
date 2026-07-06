# CodeIO Language Specification — v0 (draft)

Status: 🚧 BUILDING. This spec drives the M2 interpreter. Every construct here lowers to the
language-agnostic IR (§8), which is the same IR that foreign languages are lifted into (P6/P8).

File extension: `.cio` · Encoding: UTF-8

---

## 1. Design invariants

1. **Everything lowers to IR nodes.** Source text is one *view* of the program; the IR is the
   program. IR nodes are content-addressed (SHA-256 of canonical encoding).
2. **Code is data (P2).** Any expression can be quoted into an IR value, queried, transformed,
   and spliced back — at runtime.
3. **Tables are types (P3).** Structured data is declared, related, indexed, and queried in the
   language, checked at compile time.
4. **`ai` is an expression (P4).** Inference calls are typed expressions with schema-constrained
   outputs, explicit context budgets, and content-hash caching.
5. **Small context is enforced, not hoped for.** `ai` calls declare what they need; the runtime
   assembles it via the ContextWindowService.

## 2. Lexical structure

- Comments: `//` line, `/* */` block (nesting allowed).
- Identifiers: Unicode XID, `snake_case` values, `PascalCase` types/tables.
- Literals: `42`, `3.14`, `0xFF`, `"strings with \(interpolation)"`, `true/false`, `nil`,
  `#2026-07-06` (date), `1.5s` (duration).
- Significant tokens: `~` (quote), `~!` (unquote/splice), `?` (optional), `|>` (pipe).

## 3. Bindings, functions, types

```cio
let name = "CodeIO"                  // immutable
var count = 0                        // mutable

fn greet(who: Str) -> Str {
    "hello \(who)"
}

type Severity = enum { Info, Warn, Error }
type Point    = { x: Float, y: Float }          // record
type Result[T] = enum { Ok(T), Err(Str) }       // generics + sum types
```

- Type inference is pervasive (HM-style; the Haskell layer owns full inference).
- Pattern matching: `match sev { .Error => ..., _ => ... }`.
- Pipes: `data |> filter(...) |> map(...)`.

## 4. Tables (P3)

A table is a schema-checked, indexed, relation-aware collection — a language construct,
not a library.

```cio
table Users {
    id:      Uuid     @key
    email:   Str      @unique @index
    name:    Str
    team_id: Uuid?    @ref(Teams.id)          // enforced relation
    created: DateTime @default(now())

    @index(name, trigram)                     // hybrid search via Spotlight
}

table Teams {
    id:   Uuid @key
    name: Str  @unique
}
```

Semantics:
- `@key/@unique/@index/@ref/@default` are schema attributes enforced by the runtime.
- Every row is content-addressed; every mutation is an event (enables live queries + sync).
- Relations declared with `@ref` are navigable both directions: `user.team`, `team.users`.

### Queries

Queries are first-class, statically checked expressions:

```cio
let admins = from u in Users
             join t in Teams on u.team_id == t.id
             where t.name == "admin" && u.created > #2026-01-01
             select { u.name, u.email }

live let online = from u in Users where u.status == .Online
// `live` re-evaluates incrementally on every relevant mutation (powers IDE power-forms)
```

- `from/join/where/select/order/take` — checked against schemas at compile time;
  an unknown column or type mismatch is a compile error.
- Queries compile to plans over the index services (trigram/semantic where applicable).
- A query value is itself data (P2): it can be inspected, rewritten, and re-planned.

## 5. The `ai` primitive (P4)

```cio
// Simplest form: typed one-shot
let sentiment: Severity = ai "classify severity of this log line" with { line: log_text }

// Full form
let fix = ai {
    task:    "propose a minimal patch for the failing test",
    context: focus(test_fn) budget 2_000 tokens,     // ContextWindowService assembly
    lookup:  from e in Errors where e.test == test_fn.name take 5,
    model:   ollama("qwen3:14b") | anthropic("claude-haiku"),   // fallback chain
    output:  Patch,                                   // schema-constrained decode
    cache:   content,                                 // hash(inputs) => reuse
}
```

Semantics:
- **Typed output.** `output: T` constrains decoding to `T`'s schema; a non-conforming
  response is a retriable runtime error, never silent garbage.
- **Context is declared.** `context: focus(x) budget N tokens` asks the ContextWindowService
  for a relevance-ranked, budget-capped window around `x`. `lookup:` pulls table rows at call
  time instead of baking facts into prompts. **There is no "include the whole file" form.**
- **Caching.** `cache: content` keys on the hash of (task, context, lookups, model); identical
  calls are free. `cache: none` opts out.
- **Backends.** `ollama(...)`, `anthropic(...)`, `mesh(...)` (P7 — routes to the P2P landscape).
  `|` declares fallback order.
- `ai` expressions are ordinary expressions: composable, pipeable, quotable.

### AI line-by-line mode (M4 preview)

In the REPL, a leading `?` makes the statement an AI transform of the current scope with a
per-statement default budget:

```cio
> let rows = from u in Users where u.churned select u
> ? summarize rows as three bullet points          // ai call, scope-aware, tiny context
> ? write a test for greet                          // emits quoted code, user splices/accepts
```

## 6. Inline metaprogramming (P2)

```cio
let code = ~{ fn double(x: Int) -> Int { x * 2 } }   // quote: expr -> IR value
let n    = code.query("count nodes of kind Call")     // self-analysis API on IR
insert ~!{ code }                                      // splice IR back into the program

meta fn derive_json(t: TypeDef) -> IR {               // compile-time function
    // runs during compilation; generates serializer from the type's IR
}

@derive_json
type Config = { host: Str, port: Int }
```

- `~{}` quote / `~!{}` unquote operate on IR nodes, not text — hygienic by construction.
- The self-query API is the same IndexEntry/Spotlight machinery the IDE and AI use: a program
  can ask "who calls me", "what changed", "generate a test for X" about itself.
- `meta fn` executes at compile time in a sandboxed interpreter; its inputs and outputs are IR.

## 7. Effects & live execution (P1)

- Side effects are tracked coarsely in v0: functions are inferred `pure` or `effectful`
  (io, table-write, ai). Purity is what makes `cache: content` and live queries sound.
- Hot swap contract (M11): a top-level definition may be replaced at runtime iff its type
  signature is unchanged; the VM migrates references atomically.

## 8. The IR (the real program)

Twelve node kinds (aligned with proto/frontend/ast.proto — to be reconciled in M2):

`Literal · Ref · Call · Fn · Match · Record · TableDef · Query · AiCall · Quote · Splice · Effect`

- Canonical encoding: protobuf; node id = SHA-256(canonical bytes) — content-addressed.
- Foreign code lifted via tree-sitter (P6) maps into the same node kinds, with `Effect(opaque)`
  for constructs that don't translate; even opaque nodes are indexable and visualizable.
- The IR, not the surface syntax, is what the IDE renders (2D node graph, 3D topology) and what
  the AI addresses. Surface syntax ↔ IR round-trips losslessly for native code.

## 9. Open questions (resolve during M2)

1. Row polymorphism vs. nominal records for table projections.
2. `live` query semantics under transactions (snapshot vs. eventual).
3. `meta fn` capability sandbox: which effects (if any) are permitted at compile time.
4. Error model for `ai` output-decode retries: max attempts, backoff, cost accounting.
5. Numeric tower: Int/Float only in v0, or arbitrary precision from day one.
