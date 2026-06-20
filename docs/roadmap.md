# Knot Roadmap

Knot is a multi-language static analysis engine built around Tree-sitter parsing,
language-specific semantic adapters, a shared fact model, and sandboxed rules.

The useful product is not "a faster ESLint/Ruff clone." The useful product is a
small, embeddable policy engine for codebases that need custom rules across more
than one language.

This roadmap is intentionally narrower than the design notes. It focuses on
getting to a working vertical slice quickly, then widening only after the core
model has survived real rules.

## Product Shape

Knot should initially ship as:

- a CLI that analyzes files and prints diagnostics
- a Rust library that exposes parsing, facts, and diagnostics
- a Wasm plugin interface for all rules

Primary languages:

- Python first
- TypeScript second

Stretch language:

- Lua, only after the extension story is real

Non-goals for the first production-quality version:

- type checking
- whole-program analysis
- full IDE feature parity
- competing directly with specialized ecosystem linters
- user-distributed rule marketplaces

## Architecture

```text
Source text
  -> Tree-sitter parser
  -> language adapter
  -> semantic facts
  -> Wasm rule scheduler
  -> Wasm plugin rules
  -> diagnostics
  -> optional fixes
```

The host owns parsing, semantic analysis, scheduling, diagnostics, and fixes.
Rules consume facts. They should not walk Tree-sitter nodes directly across a
plugin boundary.

The first implementation should run rules through Wasm. That makes the ABI part
of the first vertical slice rather than a later compatibility problem.

## Core Principles

- Keep syntax language-specific.
- Keep shared facts small and boring.
- Prefer a fact database over a universal AST or IR.
- Make single-file analysis excellent before attempting cross-file analysis.
- Treat the Wasm ABI as part of the core product, not an extension layer.
- Treat Python scope analysis as the first serious correctness test.
- Do not add fix support until diagnostics and spans are trustworthy.

## Fact Model

Start with a minimal shared schema:

- `File`
- `Span`
- `Scope`
- `Binding`
- `Reference`
- `Import`
- `Call`
- `MemberAccess`
- `Literal`
- `Diagnostic`

Each fact needs:

- stable ID
- source span
- owning file
- optional parent fact or scope
- language tag

Language-specific facts are allowed, but should be introduced only when a real
rule needs them.

Initial Python-specific facts:

- `GlobalDeclaration`
- `NonlocalDeclaration`
- `Decorator`
- `ComprehensionScope`

Initial TypeScript-specific facts:

- `OptionalChain`
- `JsxElement`
- `Export`
- `TypeAnnotation`

## Milestone 0: Repository Foundation

Goal: make the project buildable and easy to extend.

Status: complete.

Tasks:

- create Rust workspace - ✅
- add core crates - ✅
- add basic CLI binary - ✅
- add snapshot-style test harness - ✅
- add fixture layout for Python and TypeScript - ✅
- add CI with format, lint, and tests - ✅
- add initial architecture documentation - ✅
- establish typed error and newtype conventions - ✅

Current crate layout:

```text
crates/
  knot-core/
  knot-cli/
  knot-parser/
  knot-facts/
  knot-runtime/
  knot-abi/
  knot-diagnostics/
```

Exit criteria:

- `cargo test --workspace` runs
- CLI accepts file paths with `knot check <paths...>`
- fixtures can assert diagnostics
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo test --workspace` run in CI

Foundation conventions:

- use `thiserror` for typed library errors
- use `anyhow` at binary/tooling boundaries
- define domain newtypes once and re-export them when another crate needs the
  same concept
- keep `RuleId` and `FileId` canonical in `knot-diagnostics` while
  `knot-runtime` and `knot-parser` re-export them

## Milestone 1: Parser Spine

Goal: parse files reliably and expose source mapping.

Status: complete.

Tasks:

- integrate Arborium-backed Tree-sitter parsing - ✅
- add Python parser - ✅
- add TypeScript/TSX parser - ✅
- represent files and line/column mapping - ✅
- expose parse errors as diagnostics - ✅
- support incremental parse internally, but do not optimize around it yet - ✅

Exit criteria:

- CLI parses Python and TypeScript files - ✅
- parse errors include useful spans - ✅
- tests cover UTF-8, multiline spans, and syntax errors - ✅

## Milestone 2: Wasm Rule Pipeline

Goal: run simple Wasm rules end to end before building deep semantics.

Status: complete.

Tasks:

- choose the initial Wasm runtime: Wasmtime - ✅
- define ABI versioning - ✅
- define fact serialization: UTF-8 JSON bytes - ✅
- define memory ownership for strings, spans, facts, and diagnostics - ✅
- define rule metadata export - ✅
- define the minimal syntax facts needed by the first rules - ✅
- define diagnostic structure - ✅
- add bundled rule registry - ✅
- add scheduler for single-file bundled rules - ✅
- add timeout and memory limits - ✅
- recover cleanly from rule failures - ✅
- add Wasm rule fixture harness - ✅
- emit human-readable diagnostics - ✅
- emit JSON diagnostics - ✅

First rules:

- Python mutable default argument - ✅
- TypeScript `debugger` - ✅
- TypeScript `console.*` - ✅

Minimal facts for those rules:

- Python function parameters and default-value literals - ✅
- TypeScript debugger statements - ✅
- TypeScript calls and member accesses - ✅

Exit criteria:

- CLI runs bundled rules by default on selected files - ✅
- diagnostics have stable rule IDs, messages, severity, and spans - ✅
- fixture tests cover all first rules - ✅
- rule failures cannot crash the host - ✅
- ABI version mismatch produces a clear error - ✅

## Milestone 3: Python Semantic Adapter

Goal: build enough Python semantics for useful policy rules.

Tasks:

- construct lexical scopes
- model module, function, class, lambda, and comprehension scopes
- collect bindings
- collect references
- resolve references to local bindings where possible
- collect imports
- handle `global` and `nonlocal`
- model decorators

Important edge cases:

- comprehension scope behavior
- class body lookup behavior
- nested closures
- assignment expressions
- `from module import *`
- alias imports
- exception handler bindings
- pattern matching bindings

Rules to validate the model:

- unused import
- undefined local reference, best-effort
- bare except

Exit criteria:

- Python unused import works on realistic fixtures
- unresolved/dynamic cases are represented explicitly
- false positives are avoided for known hard cases, even if that means declining
  to report

## Milestone 4: Diagnostics Quality And Suppression

Goal: make diagnostics usable in real projects.

Tasks:

- support project ignore files such as `.gitignore` during path discovery
- stable diagnostic IDs
- severity levels
- related spans
- rule configuration
- inline suppression comments
- file-level suppression comments
- SARIF output
- deterministic ordering

Exit criteria:

- diagnostics are stable across runs
- suppressions are tested
- CLI can produce human, JSON, and SARIF output

## Milestone 5: TypeScript Semantic Adapter

Goal: add useful TypeScript facts without becoming a type checker.

Tasks:

- parse TypeScript and TSX
- collect lexical scopes
- collect bindings and references
- collect imports and exports
- collect calls and member accesses
- model optional chaining
- model JSX elements

Rules to validate the model:

- no console
- no debugger
- restricted import
- restricted API call

Defer:

- type-aware rules
- declaration merging
- full module resolution
- prefer-const

Exit criteria:

- TypeScript rules work without type information
- JSX does not break fact extraction
- imports and exports are represented consistently

## Milestone 6: Fix Engine

Goal: support safe autofixes for narrow, well-understood cases.

Tasks:

- define text edit model
- detect overlapping edits
- apply fixes deterministically
- support dry-run fix output
- support write mode
- preserve line endings

First fixes:

- remove unused Python import
- remove TypeScript `debugger`

Exit criteria:

- fix application is snapshot-tested
- overlapping fixes are rejected safely
- CLI can show and apply fixes

## Milestone 7: Wasm Runtime Hardening

Goal: make the Wasm plugin runtime robust enough for third-party rules.

Tasks:

- stabilize the ABI after real rule feedback
- add compatibility tests across ABI versions
- add richer rule configuration
- add packaged rule loading
- add plugin discovery
- improve sandbox resource accounting
- document the guest SDK

Recommended approach:

- keep facts batch-oriented
- pass compact immutable fact snapshots
- avoid host callbacks for AST traversal
- support a narrow ABI first, then expand only for real rules

Exit criteria:

- third-party Wasm rules can be loaded from disk
- old compatible rules continue to run
- incompatible rules fail with actionable errors

## Milestone 8: Watch Mode And Editor Readiness

Goal: make the engine suitable for editor integration.

Tasks:

- file watching
- incremental reparsing
- cache invalidation
- cancellation
- debounced analysis
- machine-readable streaming output

Defer a full LSP until the engine behavior is stable under watch mode.

Exit criteria:

- repeated edits re-analyze only necessary files
- cancellation does not corrupt state
- diagnostics update deterministically

## Milestone 9: Cross-File Analysis

Goal: support organization policy rules that need project context.

Tasks:

- project index
- dependency graph
- import resolution
- cache persistence
- invalidation strategy
- cross-file fact queries

Candidate rules:

- banned internal API usage
- framework boundary violations
- deprecated SDK migration checks

Exit criteria:

- cross-file rules work incrementally
- stale facts are invalidated correctly
- project cache format is versioned

## Milestone 10: Lua Extension Spike

Goal: prove that adding a new language is straightforward.

Tasks:

- add Lua grammar
- implement minimal Lua adapter
- collect scopes, bindings, references, and calls
- add two or three demo rules

Exit criteria:

- Lua support requires no core architecture changes
- gaps in the language adapter interface are documented

## Risk Register

Largest risks:

- Python scope correctness
- span correctness under edits and fixes
- Wasm ABI churn
- rule false positives from incomplete semantics
- cross-file invalidation complexity

Mitigations:

- keep the first ABI narrow
- use fixtures heavily
- represent uncertainty explicitly in facts
- prefer not reporting over noisy reporting
- keep cross-file analysis out of the first usable version

## First Vertical Slice

The first useful demo should be:

```text
knot check examples/
```

It should:

- parse Python and TypeScript
- run Wasm plugin rules
- print deterministic diagnostics
- support JSON output
- have fixture tests

The minimum rule set:

- Python mutable default argument
- TypeScript `debugger`
- TypeScript `console.*`

This proves the pipeline without requiring the hardest semantic work upfront.

## Near-Term Task Order

1. Add source file loading and deterministic path discovery - ✅.
2. Add line/column source mapping tests for UTF-8 and multiline spans - ✅.
3. Integrate Arborium-backed Tree-sitter parsing and add Python parsing - ✅.
4. Expose Python parse errors as diagnostics - ✅.
5. Add TypeScript and TSX parsing - ✅.
6. Expose TypeScript/TSX parse errors as diagnostics - ✅.
7. Wire parser diagnostics into `knot check` - ✅.
8. Expand fixture snapshots for valid files, syntax errors, UTF-8, and
   multiline spans - ✅.
9. Define the first Wasm ABI - ✅.
10. Implement the Wasm runtime, rule registry, and scheduler - ✅.
11. Implement the first three syntax-oriented rules as Wasm fixtures - ✅.
12. Add JSON output - ✅.
13. Start Python scope and binding facts.
