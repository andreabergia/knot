# Knot Design Notes

## Vision

A fast, extensible static-analysis platform supporting multiple languages via:

- Tree-sitter parsing
- Rust native core
- Wasm plugin rules
- Incremental analysis
- Shared semantic facts
- Language-specific semantic adapters

Primary targets:

- Python
- TypeScript

Stretch goal:

- Lua (simple grammar; validates extensibility story)

Long-term goal:

- Organization-wide policy enforcement
- Portable custom rules
- Editor + CI integration
- Sandboxed rule ecosystem

Non-goal:

- Outperforming specialized tools like Ruff, Biome, or Oxlint in their own ecosystems.

---

# Strategic Positioning

This framework is **not** primarily a replacement for:

- Ruff
- Biome
- ESLint
- Clippy
- staticcheck

Instead, it is best positioned as:

```text
Semgrep / CodeQL-lite
but:
- incremental
- embeddable
- editor-friendly
- Wasm-plugin-based
- multi-language
```

Best use cases:

- organization-wide policies across polyglot monorepos
- security rules
- API governance
- framework usage enforcement
- architectural constraints
- internal SDK migration checks
- LLM-assisted authoring of custom rules

### Why Python + TypeScript

Python is the primary semantic challenge: closures, comprehension scopes, decorators, `nonlocal`,
and `import *` make it a thorough test of the semantic layer. Interviewers understand Python rules
without context-switching.

TypeScript is the right second language because:

- type-aware rule tooling remains operationally heavy across the ecosystem
- optional chaining, JSX, imports/exports, and module systems provide genuine semantic complexity
- large addressable audience; real unsolved problems

Go and Rust are excluded: both have mature, well-maintained tooling ecosystems with little room
for meaningful differentiation.

Lua is a stretch goal. Its grammar is small, Tree-sitter support is excellent (driven by the
Neovim ecosystem), and tooling is nearly nonexistent. Adding Lua later would serve
as a concrete demo of how easy it is to extend the framework to a new language.

---

# Core Architecture

```text
Source Files
    ↓
Tree-sitter Parsers
    ↓
Language Adapters
    ↓
Semantic Facts
    ↓
Wasm Rules
    ↓
Diagnostics + Fixes
```

---

# Key Architectural Decisions

## Parsing

Use Tree-sitter for:

- incremental parsing
- multi-language support
- editor integration
- robust syntax recovery

Tree-sitter provides CSTs, not semantic analysis.

---

## Runtime

Use Rust for:

- parser orchestration
- caching
- concurrency
- semantic analysis
- scheduling
- diagnostics
- fix application

Rules run as Wasm plugins. The Rust host owns all expensive shared analysis and passes compact,
versioned fact snapshots to sandboxed rules.

---

# Rule Model

Rules consume semantic facts instead of traversing raw ASTs.

Guest SDKs can expose an ergonomic shape like this, but the host/plugin contract is the Wasm ABI,
not an in-process Rust trait:

```rust
trait Rule {
    fn meta() -> RuleMeta;
    fn interests() -> Vec<Interest>;
    fn check(ctx: RuleCtx, event: Event) -> Vec<Diagnostic>;
}
```

The `Interest`/event model is a push/subscription system: the host dispatches fact events only
to rules that have opted in. Because all rules cross the Wasm boundary, the event taxonomy, fact
serialization, memory ownership, and ABI versioning must be designed as part of the first runnable
rule pipeline.

Important principle:

```text
Do not expose fine-grained AST traversal across the Wasm boundary.
```

The host computes expensive shared facts once.

---

# Semantic Model

Avoid a universal AST or compiler-grade IR.

Use:

```text
Tree-sitter CST
→ language-specific semantic adapter
→ shared fact schema
→ optional language-specific facts
```

---

# Shared Facts

Examples:

```rust
Binding
Reference
Import
Call
MemberAccess
Literal
Scope
```

These are language-agnostic enough to support many useful rules.

---

# Language-Specific Facts

## Python

```rust
GlobalDeclaration
NonlocalDeclaration
Decorator
ComprehensionScope
```

## TypeScript

```rust
HoistedVar
OptionalChain
JsxElement
ExportDefault
TypeAnnotation
InterfaceDeclaration
```

---

# Wasm ABI

The rule ABI across the Wasm boundary is a first-class design concern and must be resolved before
the first rule pipeline is considered real. Key decisions required:

- Serialization format for facts (candidates: Flatbuffers, Cap'n Proto, custom encoding)
- Memory model: who owns allocations, how strings and spans are passed
- Error handling and sandboxed panic recovery
- Versioning strategy for backward-compatible rule evolution

The "good model" (host computes facts once, rules consume compact structured data) is correct,
but the concrete wire format must be pinned early to avoid breaking changes once rules are
distributed.

---

# Why Not MLIR?

MLIR is likely overkill.

This project is source-analysis-oriented, not compiler-optimization-oriented.

Problems with adopting MLIR:

- excessive abstraction cost
- difficult source preservation
- poor fit for autofixes/comments/trivia
- large semantic modeling burden
- unnecessary complexity early on

Preferred approach:

```text
Do not build "the IR."
Build a fact database.
```

---

# Example Rules

## Python — Unused Import

Uses:

- import facts
- reference resolution

Produces:

- warning
- optional delete fix

---

## Python — Mutable Default Argument

Mostly syntax-based.

Detect:

```python
def f(x=[]):
```

---

## TypeScript — no-console

Detects calls to:

```js
console.log()
```

with allow-list support.

---

## TypeScript — no-debugger

Simple syntax rule:

```js
debugger;
```

---

## TypeScript — prefer-const

Requires:

- binding analysis
- write tracking across all assignments
- likely limited CFG support

---

# Performance Considerations

The biggest performance risk is the Wasm boundary.

Bad model:

```text
rule → host call → AST child
rule → host call → parent
rule → host call → text
```

Good model:

```text
host computes facts once
rules consume compact structured data
```

Optimization priorities:

- incremental parsing
- immutable AST sharing
- batched fact access
- parallel scheduling
- persistent caches
- zero-copy spans

---

# Limitations

## Generic analysis has limits

Semantics vary significantly across languages.

---

## Type-aware analysis is hard

Requires deep ecosystem integration:

- TypeScript compiler API
- pyright/mypy
- build systems
- module resolution

---

## Autofixes are harder than diagnostics

Need:

- formatting preservation
- conflict resolution
- safe edit merging

---

## Cross-file analysis is expensive

Requires:

- indexing
- invalidation
- dependency graphs
- project caches

**Important:** The initial implementation should focus on single-file rules. Several stated use
cases (API governance, SDK migration, architectural constraints) require cross-file resolution and
should wait until parsing, facts, diagnostics, and invalidation are stable.

---

# AI-Assisted Rules

AI-generated rules are a reasonable fit for this architecture.

Why:

- stable semantic fact API
- portable rule format
- constrained execution model
- easy rule synthesis

Ideal interaction:

```rust
ctx.bindings()
ctx.references()
ctx.calls()
ctx.imports()
```

This is easier for LLMs than raw AST traversal. However, API ergonomics do not solve rule
*correctness* — false positive rates still depend on how carefully the rule author (human or LLM)
reasons about when a pattern is actually safe to flag.

---

# Biggest Technical Risk

The hardest problem is not parsing.

It is:

```text
building a high-quality semantic layer
that handles language edge cases
without becoming slow
```

That is the true core of the system.

---

# Recommended Philosophy

```text
Keep the parser generic.
Keep semantics language-specific.
Keep rules mostly language-agnostic when possible.
Avoid premature universal abstractions.
Optimize the host and minimize Wasm boundary crossings.
Pin the Wasm ABI early.
Start with single-file analysis; add cross-file analysis only after the core engine is stable.
```
