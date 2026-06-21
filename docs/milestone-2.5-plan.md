# Milestone 2.5: Rule SDK — Implementation Plan

Status: ✅ complete (steps 1-5), ⬜ CI (step 6), ⬜ docs (step 7)

Goal: provide an ergonomic Rust SDK so rule authors can write rules without
touching raw Wasm or the ABI directly. The raw ABI remains the ground truth
underneath; the SDK is a developer-experience layer.

This is an intentionally simple first version. It will evolve as the fact model
grows (Milestones 3 and 5) and the ABI matures. Milestone 7 will revisit the
SDK for stabilization alongside the ABI.

## Context

Before this milestone:

- The three existing "Wasm rules" were hand-written in WebAssembly Text format
  (`.wat`) under `crates/knot-core/rules/`, compiled to `.wasm` by
  `knot-core/build.rs` via the `wat` crate, and embedded into the host with
  `include_bytes!` in `crates/knot-core/src/bundled_rules.rs`.
- There was no `knot-sdk` crate, no `wasm32-unknown-unknown` target setup, no
  `.cargo/config.toml`, no macros, no `wit`, and no rule sub-crate.
- The ABI is small and exactly pinned: `ABI_VERSION = 1` (exact match), five
  exports (`memory`, `knot_alloc`, `knot_dealloc`, `knot_metadata`,
  `knot_check`), UTF-8 JSON wire types in `knot-abi`, return convention
  `(ptr << 32) | len`, batched facts per file, no host callbacks.
- `knot-diagnostics` and `knot-abi` are wasm-clean (only `std::{fmt, cmp, path}`
  and `serde`/`serde_json`), so `knot-sdk → knot-abi → knot-diagnostics`
  compiles to `wasm32-unknown-unknown` with no crate split required.

So milestone 2.5 is greenfield on the guest side: we re-implement the three
rules in Rust on top of a new SDK, and replace the WAT → `build.rs` →
`include_bytes!` pipeline with a Rust → wasm pipeline.

## Decisions

1. **Rule distribution**: static embed via xtask. Bundled rules stay embedded;
   `knot check` remains self-contained. The runtime already accepts raw
   `&[u8]`, so filesystem-based third-party loading is a small additive change
   in Milestone 7 — no ABI/runtime changes will be needed then.
2. **Workspace shape**: a separate `rules/` Cargo workspace for guest crates.
   Keeps `cargo test --workspace` (root) from ever trying to build guest crates
   for the host. `knot-sdk` lives in the root workspace so it's tested on the
   host normally; the rules workspace consumes it via a path dep.
3. **One crate per language, one `[[bin]]` per rule**: shared rule logic lives
   in the crate's `lib.rs`; each rule is a `[[bin]]` shim that calls the
   `register!` macro. `cargo build -p rules-ts --target
   wasm32-unknown-unknown --release` produces one `.wasm` per `[[bin]]`.
4. **Macro strategy**: declarative `macro_rules! register!` living inside
   `knot-sdk` (no extra proc-macro crate). Less ceremony, faster to land the
   three migrations. A proc-macro can replace it in Milestone 7 once the API
   stabilizes.
5. **Type sharing**: `knot-sdk` depends on `knot-abi` directly (which depends
   on `knot-diagnostics`). Both are wasm-clean, so no premature split into a
   wire-only sub-crate.
6. **Surface**: re-export the wire types from `knot-abi` and domain types from
   `knot-diagnostics` with thin convenience constructors. No deep wrapper
   hierarchy — ergonomics will grow as real rules demand them in Milestones 3
   and 5.
7. **WAT rules**: deleted outright once the migrated rules pass the existing
   fixture/snapshot tests. The git history preserves them as a reference.
8. **Host-side registry**: `bundled_rules.rs` stays hand-maintained (three
   lines). The xtask produces bytes; the registry points at them. Code-gen of
   the registry is deferred to Milestone 7.
9. **Smoke rule**: dropped. The "~10 lines" exit criterion is demonstrated by
   the migrated `ts-debugger` rule's `check` body (~5 lines) plus the
   mechanical-add story documented in the roadmap.

## Workspace Shape

```text
knot-lint/
  Cargo.toml             (root workspace — adds knot-sdk, knot-xtask as members) ✅
  crates/
    knot-sdk/            (NEW — host-testable, also compiles to wasm via path dep) ✅
    knot-xtask/          (NEW — host-only bin that builds rules) ✅
    knot-core/           (build.rs shrinks to a guard; bundled_rules.rs points
                          at rules/build/) ✅
      rules/build/       (NEW, gitignored — xtask writes .wasm here) ✅
  rules/                 (NEW — separate Cargo workspace, guest crates only) ✅
    Cargo.toml           ✅
    ts/                  (NEW crate — ts-debugger and ts-console rules) ✅
    python/              (NEW crate — py-mutable-default-arg rule) ✅
```

- Root workspace `members` adds `crates/knot-sdk` and `crates/knot-xtask`.
  Resolver stays `3`. ✅
- `rules/Cargo.toml` is a separate workspace with members `ts` and `python`.
  Its `[workspace.dependencies]` declares `knot-sdk = { path =
  "../crates/knot-sdk" }` plus shared `serde`/`serde_json`. ✅
- `rules/ts/Cargo.toml` has `[[bin]] name = "ts-debugger"` and `[[bin]] name =
  "ts-console"`. Each `src/bin/<name>.rs` is a 3-line shim: ✅

  ```rust
  use rules_ts::debugger::DebuggerRule;
  knot_sdk::register!(DebuggerRule);
  ```

- Shared rule logic lives in `rules/ts/src/lib.rs` (or `src/debugger.rs` /
  `src/console.rs`). ✅
- `rules/python/Cargo.toml` has one `[[bin]] name = "py-mutable-default-arg"`. ✅
- Each `[[bin]]` needs a `fn main()`. The `register!` macro emits a no-op
  `fn main() {}` automatically so bins link cleanly on
  `wasm32-unknown-unknown` (the host never calls `main`; it calls the four ABI
  exports). ✅

## `knot-sdk` Crate Surface

`crates/knot-sdk/src/lib.rs`: ✅

- Re-exports `knot_abi::{FactPayload, PythonFactPayload, TypeScriptFactPayload,
  RuleInput, DiagnosticPayload, SpanPayload, SeverityPayload, LiteralPayload,
  RuleMetadata, AbiVersion, ABI_VERSION}`. Adds a thin
  `DiagnosticPayload::new(rule_id, severity, message, span)` constructor so a
  rule doesn't repeat field names. ✅
  > Deviation: the constructor lives on `DiagnosticPayload` in `knot-abi`
  > rather than in `knot-sdk`. Rust's orphan rules forbid an inherent impl on
  > a foreign type, so the impl must live next to the type definition. The
  > SDK re-exports the type, so `knot_sdk::DiagnosticPayload::new(...)` is
  > the call site rule authors use; only the impl block's crate differs.
  > Also, the `Severity`/`Span`/etc. host-type re-exports via
  > `knot_diagnostics` were dropped — no rule or SDK test consumed them.
- `pub trait Rule: Default` with `fn metadata() -> RuleMetadata` and `fn
  check(&self, ctx: &RuleContext) -> Vec<DiagnosticPayload>`. The `Default`
  bound reflects that rules are stateless zero-sized config structs; config
  support lands in Milestone 7 via `Rule::Config: Default` + an extended
  `register!`.
  > Deviation: `metadata()` is an associated function rather than `const
  > METADATA` because `String` in `const` context is not stable on Rust 1.95
  > (`to_owned()` is not a const fn). This is identical at the call site and
  > can be migrated to a `const` if/when const traits stabilize.
- `pub struct RuleContext<'a>` wrapping `&'a RuleInput`, with `.facts() ->
  &[FactPayload]`, `.python_facts() -> impl Iterator<Item =
  &PythonFactPayload>`, `.typescript_facts() -> ...`, and
  `.syntax_diagnostics() -> &[DiagnosticPayload]`. Just thin views — no
  wrapper hierarchy. ✅
- `pub macro register!($ty:ty)` — a declarative `macro_rules!` living in
  `knot-sdk` (no extra crate). It emits: ✅
  - A static bump arena (`static mut HEAP: [u8; N]` + cursor; wasm is
    single-threaded under `wasm32-unknown-unknown`, so no atomics needed; the
    1 MiB arena is well under Wasmtime's 16 MiB memory cap).
  - `#[unsafe(no_mangle)] pub extern "C" fn knot_alloc(len: u32) -> u32`
    (bump, returns absolute addresses: `HEAP.as_ptr() + offset`).
    > Deviation: `#[no_mangle]` → `#[unsafe(no_mangle)]` required by edition
    > 2024 for extern functions. Absolute addresses required because the host
    > reads from the exported `memory`, not a Rust-relative offset.
  - `#[unsafe(no_mangle)] pub extern "C" fn knot_dealloc(_ptr: u32, _len: u32)`
    (no-op, matching today's guests).
  - `#[unsafe(no_mangle)] pub extern "C" fn knot_metadata() -> u64` —
    `encode_json(&<$ty as Rule>::metadata())`, copy bytes into the arena,
    return `(ptr << 32) | len`.
  - `#[unsafe(no_mangle)] pub extern "C" fn knot_check(ptr: u32, len: u32) -> u64` —
    `decode_json::<RuleInput>` from the input slice, construct
    `<&$ty>::default()`, call `check(ctx)`, `encode_json(&result)`, return
    packed. Internally delegates to a pub `run_check::<R>()` helper so host
    tests can exercise the glue without wasm.
  - `fn main() {}` so the `[[bin]]` links.
- Panic and unwinding on wasm: ✅
  > Deviation from plan: `panic_handler` and `eh_personality` are **not**
  > defined in the SDK library because `std` already provides them on
  > `wasm32-unknown-unknown`. Rule crates set `panic = "abort"` in
  > `[profile.release]` (via the `rules/` workspace-level profile), which
  > avoids the `eh_personality` requirement. This is simpler and uses the
  > standard library's defaults.
- Host unit test: a fake `Rule` impl round-trips through `run_check` to verify
  the generated glue (deserialization, dispatch, reserialization) without
  needing wasm. ✅

## `knot-xtask` Crate

`crates/knot-xtask/` — a normal host bin in the root workspace, run via
`cargo run -p knot-xtask -- build-rules`. ✅

`build-rules` does, in order:

1. `cargo build --manifest-path rules/Cargo.toml --target
   wasm32-unknown-unknown --release` via `std::process::Command`. ✅
2. Finds every `*.wasm` under
   `rules/target/wasm32-unknown-unknown/release/` whose filename matches a
   rule id (filtering out `*.d.wasm`/deps).
   > Deviation: currently uses a hardcoded list of three rule IDs rather
   > than glob-scanning the wasm directory. The set is small enough that
   > scanning adds unnecessary complexity at this stage.
3. Copies each to `crates/knot-core/rules/build/<name>.wasm` (creating the
   directory if missing). ✅
4. Prints a summary. ✅

This is the only place that shells out to cargo — it's a separate process, not
a build.rs, so no cargo-in-cargo recursion.

## Host Integration ✅

- `crates/knot-core/rules/build/` is gitignored; the root `.gitignore` is
  updated. ✅
- `knot-core/build.rs` shrinks from "compile WAT" to: assert each expected
  file in `rules/build/` exists; emit
  `cargo::rerun-if-changed=rules/build/<name>.wasm` for each. If any is
  missing, `panic!("run \`cargo run -p knot-xtask -- build-rules\` first")`.
  The `wat` build-dep is removed from `knot-core`. ✅
- `bundled_rules.rs` keeps the hand-maintained `&[BundledRule]` const, three
  entries, pointing at `include_bytes!("../rules/build/<name>.wasm")`. The
  existing `bundled_rule_metadata_matches_registration` test stays as the
  id ↔ metadata gate, now exercised against SDK-generated metadata. ✅
  > Deviation: `include_bytes!` path is `"../rules/build/…"` (relative to
  > `src/bundled_rules.rs`) instead of the plan's `"rules/build/…"` because
  > `include_bytes!` is relative to the source file, not the crate root.
- The three `.wat` files under `crates/knot-core/rules/` are deleted. ✅

## Execution Order

Red → green per the repo's TDD rule. Each step lands green before the next.

1. ✅ **Add `knot-sdk`** (crate, root workspace member, depends on `knot-abi` +
   `knot-diagnostics` via workspace deps). Add `pub trait Rule`,
   `RuleContext`, thin constructors, `register!` macro, `run_check` helper.
   Add a host unit test exercising `run_check` against a fake `Rule`.
   Verify `cargo test -p knot-sdk` green. Verify `cargo build -p knot-sdk
   --target wasm32-unknown-unknown` compiles (smoke check the crate is
   wasm-clean).
2. ✅ **Add `rules/` workspace** with two crates (`ts`, `python`). Each depends
   on `knot-sdk`. Implement all three `Rule` impls in their `lib.rs` with
   `[[bin]]` shims calling `register!`. Not yet wired to the host.
3. ✅ **Add `knot-xtask`** (crate, root workspace member). Implement
   `build-rules`.
4. ✅ **Run xtask** to produce `.wasm` artifacts in
   `crates/knot-core/rules/build/`.
5. ✅ **Migrate `bundled_rules.rs`** to the new `include_bytes!` paths; shrink
   `knot-core/build.rs` to the guard; delete the three `.wat` files and the
   `wat` build-dep from `knot-core`. Run `cargo test --workspace`: all 52
   tests pass — the five `check_paths_*` tests, four insta snapshots, and
   all other existing tests pass unchanged.
6. ⬜ **Update CI** (`.github/workflows/ci.yml`): add `rustup target add
   wasm32-unknown-unknown`, run `cargo run -p knot-xtask -- build-rules`
   before `cargo fmt --check` / `clippy` / `cargo test --workspace`.
7. ⬜ **Update docs**: `docs/roadmap.md` (milestone 2.5 status → complete; note
   the ~10-line claim is demonstrated by the migrated `ts-debugger` `check`
   body); `docs/ARCHITECTURE.md` (fix the `knot_rule_metadata` →
   `knot_metadata` drift at `crates/knot-abi/src/lib.rs:31`; document the SDK
   crate, the `rules/` workspace, and the xtask build flow).

## Exit Criteria

- ✅ **"writing a new rule requires no direct ABI interaction"** — the
  `register!` macro hides all four exports, the bump allocator, and the JSON
  glue. A rule author writes only a `Rule` impl + a 3-line `[[bin]]` shim.
- ✅ **"all three existing rules pass their fixture tests when reimplemented via
  the SDK"** — step 5 gate (byte-exact diagnostics, all 52 tests pass
  unchanged).
- ✅ **"a new trivial rule can be written in ~10 lines of Rust"** — demonstrated
  by the migrated `ts-debugger` rule's `check` body (5 lines). Adding a new
  rule is one `Rule` impl, one `[[bin]]` shim, one `bundled_rules.rs` line,
  one fixture — no SDK or ABI work.
- ✅ **"the SDK crate compiles to `wasm32-unknown-unknown`"** — step 1 smoke
  build; all three rule crates transitively rebuild `knot-sdk` for wasm.

## Implementation Deviations

1. **`Rule::metadata()` instead of `const METADATA`**: `String` in `const`
   context is not stable on Rust 1.95 (`to_owned()` is not const). An
   associated function achieves the same result. Can be migrated to `const`
   if/when const traits stabilize.
2. **No `panic_handler` or `eh_personality` in SDK**: `std` provides these on
   `wasm32-unknown-unknown`. Rule crates set `panic = "abort"` via the
   workspace-level `[profile.release]`, which avoids the `eh_personality`
   requirement.
3. **`#[unsafe(no_mangle)]` instead of `#[no_mangle]`**: Edition 2024 requires
   the `unsafe` qualifier on `no_mangle` for `extern` functions.
4. **`knot_alloc` returns absolute addresses**: `HEAP.as_ptr() + offset` rather
   than just the offset. The host reads from the exported `memory` at absolute
   addresses, not Rust-relative offsets.
5. **`include_bytes!` paths use `"../rules/build/…"`**: Relative to the source
   file (`src/bundled_rules.rs`), not the crate root.
6. **`DiagnosticPayload::new` lives in `knot-abi`, not `knot-sdk`**: Rust's
   orphan rules forbid an inherent impl on a foreign type. The SDK re-exports
   the type, so the call site (`knot_sdk::DiagnosticPayload::new(...)`) is
   unchanged; only the impl block's crate differs.
7. **`knot_alloc` bounds-checks the arena**: the plan described a plain bump
   allocator, but the implementation panics if `HEAP_CURSOR + len > HEAP.len()`
   to prevent a large host input from overflowing the 1 MiB static and
   corrupting adjacent linear memory. With `panic = "abort"` the panic traps
   cleanly and the host classifies it as a `GuestTrap`.
8. **Fresh-instance-per-call assumption**: the bump cursor never resets. This
   is sound only because `WasmRuntime` creates a new `RuleInstance` for each
   `metadata()`/`check()` call. Documented in a comment in the `register!`
   macro so a future refactorer introducing instance reuse doesn't exhaust
   the arena after ~1 MiB of cumulative allocations.