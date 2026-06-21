# Knot Architecture

Knot is a multi-language static analysis engine. The host owns parsing,
semantic facts, rule scheduling, diagnostics, and eventually fixes. Rules run as
sandboxed Wasm plugins and consume facts produced by the host.

```text
CLI paths
  -> core orchestration
  -> Tree-sitter parsers
  -> language adapters
  -> shared facts
  -> Wasm runtime
  -> plugin rules
  -> diagnostics
  -> CLI output
```

## Crates

`knot-cli` is the command-line app. It parses commands like `knot check <paths>`,
calls into `knot-core`, and renders diagnostics for users.

`knot-core` is the orchestration layer. It coordinates path discovery, parsing,
fact extraction, rule execution, diagnostic collection, and deterministic
ordering.

`knot-parser` owns source files, Tree-sitter parsers, parse trees, parse errors,
and source mapping from byte offsets to line and column positions.

`knot-facts` defines the shared fact model consumed by rules: scopes, bindings,
references, imports, calls, member accesses, literals, and language-specific
extensions when a real rule needs them.

`knot-diagnostics` defines diagnostic primitives: severity, spans, file
locations, messages, rule IDs, and ordering.

`knot-abi` defines the host/plugin contract. It should contain ABI versions,
export names, wire types, serialization rules, and compatibility checks. It must
not depend on a specific Wasm runtime.

`knot-runtime` implements the host side of Wasm execution. It loads plugins,
validates metadata through `knot-abi`, moves data across Wasm memory, calls rule
exports, enforces resource limits, and converts plugin output into diagnostics.

`knot-sdk` is the guest-side Rust SDK for writing Wasm rules. It provides the
`Rule` trait, a `RuleContext` for accessing facts and syntax diagnostics, and
a `register!` macro that generates all four ABI exports (`knot_alloc`,
`knot_dealloc`, `knot_metadata`, `knot_check`), a bump allocator, and the JSON
serialization glue transparently. Rule authors write only a `Rule` impl plus a
3-line `[[bin]]` shim — no direct ABI interaction. The SDK compiles for both the
host (for testing) and `wasm32-unknown-unknown` (for rules).

`knot-xtask` is a host-only tooling binary. Its `build-rules` command builds the
`rules/` workspace for `wasm32-unknown-unknown --release` and copies the
resulting `.wasm` artifacts to `crates/knot-core/rules/build/` for embedding.
Run via `cargo run -p knot-xtask -- build-rules`.

Rules live in a separate `rules/` Cargo workspace to keep guest (wasm) crates
out of the root workspace's `cargo test --workspace`. Each language has its own
crate (e.g. `rules-ts`, `rules-python`), and each rule is a `[[bin]]` shim
that calls `knot_sdk::register!`. Shared rule logic lives in the crate's
`lib.rs`.

## ABI And Runtime Boundary

`knot-abi` is the agreement between host and plugins. `knot-runtime` is the
implementation that runs plugins using that agreement.

For example, `knot-abi` can define:

```rust
pub const ABI_VERSION: AbiVersion = AbiVersion::new(1);
pub const EXPORT_METADATA: &str = "knot_metadata";
```

Then `knot-runtime` can load a Wasm module, look up `EXPORT_METADATA`, decode
the metadata, and reject plugins that do not support `ABI_VERSION`.

This keeps ABI documentation, guest SDKs, and compatibility tests independent
from any chosen Wasm engine.

## Design Rules

- Keep the CLI thin.
- Keep shared facts small and stable.
- Keep syntax handling language-specific.
- Do not expose Tree-sitter nodes across the plugin boundary.
- Run rules through Wasm from the first vertical slice.
- Prefer deterministic diagnostics over clever output.
- Add language-specific facts only when needed by concrete rules.
