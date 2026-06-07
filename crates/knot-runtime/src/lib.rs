use knot_abi::{
    AbiError, DiagnosticPayload, EXPORT_ALLOC, EXPORT_CHECK, EXPORT_DEALLOC, EXPORT_MEMORY,
    EXPORT_METADATA, RuleInput, RuleMetadata, decode_json, encode_json,
};

pub use knot_diagnostics::RuleId;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("failed to configure Wasm runtime: {0}")]
    Configuration(String),
    #[error("failed to compile Wasm module: {0}")]
    Compile(String),
    #[error("failed to instantiate Wasm module: {0}")]
    Instantiate(String),
    #[error("missing Wasm export: {0}")]
    MissingExport(&'static str),
    #[error("failed to call Wasm export {export}: {message}")]
    GuestCall {
        export: &'static str,
        message: String,
    },
    #[error("Wasm export {export} trapped: {message}")]
    GuestTrap {
        export: &'static str,
        message: String,
    },
    #[error("Wasm export {export} exhausted its fuel budget")]
    FuelExhausted { export: &'static str },
    #[error("Wasm export {export} exceeded its memory limit")]
    MemoryLimitExceeded { export: &'static str },
    #[error("failed to access guest memory: {0}")]
    Memory(String),
    #[error("{0}")]
    Abi(#[from] AbiError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeLimits {
    pub fuel: u64,
    pub memory_bytes: usize,
}

impl Default for RuntimeLimits {
    fn default() -> Self {
        Self {
            fuel: 1_000_000,
            memory_bytes: 16 * 1024 * 1024,
        }
    }
}

pub struct WasmRuntime {
    engine: wasmtime::Engine,
    limits: RuntimeLimits,
}

impl WasmRuntime {
    pub fn new() -> Self {
        Self::with_limits(RuntimeLimits::default()).expect("default runtime configuration is valid")
    }

    pub fn with_limits(limits: RuntimeLimits) -> Result<Self, RuntimeError> {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        let engine = wasmtime::Engine::new(&config)
            .map_err(|error| RuntimeError::Configuration(error.to_string()))?;
        Ok(Self { engine, limits })
    }

    pub fn metadata(&self, wasm: &[u8]) -> Result<RuleMetadata, RuntimeError> {
        let mut instance = RuleInstance::new(&self.engine, wasm, self.limits)?;
        let bytes = instance.call_metadata()?;
        let metadata: RuleMetadata = decode_json(&bytes)?;
        metadata.validate_abi()?;
        Ok(metadata)
    }

    pub fn check(
        &self,
        wasm: &[u8],
        input: &RuleInput,
    ) -> Result<Vec<DiagnosticPayload>, RuntimeError> {
        let mut instance = RuleInstance::new(&self.engine, wasm, self.limits)?;
        let metadata_bytes = instance.call_metadata()?;
        let metadata: RuleMetadata = decode_json(&metadata_bytes)?;
        metadata.validate_abi()?;

        let input_bytes = encode_json(input)?;
        let output_bytes = instance.call_check(&input_bytes)?;
        Ok(decode_json(&output_bytes)?)
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new()
    }
}

struct StoreState {
    limits: wasmtime::StoreLimits,
}

struct RuleInstance {
    store: wasmtime::Store<StoreState>,
    memory: wasmtime::Memory,
    alloc: wasmtime::TypedFunc<u32, u32>,
    dealloc: wasmtime::TypedFunc<(u32, u32), ()>,
    metadata: wasmtime::TypedFunc<(), u64>,
    check: wasmtime::TypedFunc<(u32, u32), u64>,
}

impl RuleInstance {
    fn new(
        engine: &wasmtime::Engine,
        wasm: &[u8],
        limits: RuntimeLimits,
    ) -> Result<Self, RuntimeError> {
        let module = wasmtime::Module::new(engine, wasm)
            .map_err(|error| RuntimeError::Compile(error.to_string()))?;
        let store_limits = wasmtime::StoreLimitsBuilder::new()
            .memory_size(limits.memory_bytes)
            .trap_on_grow_failure(true)
            .build();
        let mut store = wasmtime::Store::new(
            engine,
            StoreState {
                limits: store_limits,
            },
        );
        store.limiter(|state| &mut state.limits);
        store
            .set_fuel(limits.fuel)
            .map_err(|error| RuntimeError::Configuration(error.to_string()))?;
        let instance = wasmtime::Instance::new(&mut store, &module, &[])
            .map_err(|error| RuntimeError::Instantiate(error.to_string()))?;
        let memory = instance
            .get_memory(&mut store, EXPORT_MEMORY)
            .ok_or(RuntimeError::MissingExport(EXPORT_MEMORY))?;
        let alloc = instance
            .get_typed_func::<u32, u32>(&mut store, EXPORT_ALLOC)
            .map_err(|_| RuntimeError::MissingExport(EXPORT_ALLOC))?;
        let dealloc = instance
            .get_typed_func::<(u32, u32), ()>(&mut store, EXPORT_DEALLOC)
            .map_err(|_| RuntimeError::MissingExport(EXPORT_DEALLOC))?;
        let metadata = instance
            .get_typed_func::<(), u64>(&mut store, EXPORT_METADATA)
            .map_err(|_| RuntimeError::MissingExport(EXPORT_METADATA))?;
        let check = instance
            .get_typed_func::<(u32, u32), u64>(&mut store, EXPORT_CHECK)
            .map_err(|_| RuntimeError::MissingExport(EXPORT_CHECK))?;

        Ok(Self {
            store,
            memory,
            alloc,
            dealloc,
            metadata,
            check,
        })
    }

    fn call_metadata(&mut self) -> Result<Vec<u8>, RuntimeError> {
        let packed = self
            .metadata
            .call(&mut self.store, ())
            .map_err(|error| classify_guest_error(EXPORT_METADATA, error))?;
        self.read_packed(packed)
    }

    fn call_check(&mut self, input: &[u8]) -> Result<Vec<u8>, RuntimeError> {
        let ptr = self.call_alloc(input.len())?;
        self.memory
            .write(&mut self.store, ptr as usize, input)
            .map_err(|error| RuntimeError::Memory(error.to_string()))?;
        let packed = self
            .check
            .call(&mut self.store, (ptr, input.len() as u32))
            .map_err(|error| classify_guest_error(EXPORT_CHECK, error))?;
        self.call_dealloc(ptr, input.len() as u32)?;
        self.read_packed(packed)
    }

    fn call_alloc(&mut self, len: usize) -> Result<u32, RuntimeError> {
        let len =
            u32::try_from(len).map_err(|_| RuntimeError::Memory("payload too large".into()))?;
        self.alloc
            .call(&mut self.store, len)
            .map_err(|error| classify_guest_error(EXPORT_ALLOC, error))
    }

    fn call_dealloc(&mut self, ptr: u32, len: u32) -> Result<(), RuntimeError> {
        self.dealloc
            .call(&mut self.store, (ptr, len))
            .map_err(|error| classify_guest_error(EXPORT_DEALLOC, error))
    }

    fn read_packed(&mut self, packed: u64) -> Result<Vec<u8>, RuntimeError> {
        let ptr = (packed >> 32) as u32;
        let len = packed as u32;
        let end = (ptr as usize)
            .checked_add(len as usize)
            .ok_or_else(|| RuntimeError::Memory("guest output range overflowed".to_owned()))?;
        if end > self.memory.data_size(&self.store) {
            return Err(RuntimeError::Memory(
                "guest output range exceeds guest memory".to_owned(),
            ));
        }
        let mut bytes = vec![0; len as usize];
        self.memory
            .read(&self.store, ptr as usize, &mut bytes)
            .map_err(|error| RuntimeError::Memory(error.to_string()))?;
        Ok(bytes)
    }
}

fn classify_guest_error(export: &'static str, error: wasmtime::Error) -> RuntimeError {
    if error.downcast_ref::<wasmtime::Trap>() == Some(&wasmtime::Trap::OutOfFuel) {
        return RuntimeError::FuelExhausted { export };
    }

    let memory_limit_exceeded = error.chain().any(|cause| {
        cause
            .to_string()
            .contains("forcing trap when growing memory")
    });
    let message = error.to_string();
    if memory_limit_exceeded {
        RuntimeError::MemoryLimitExceeded { export }
    } else if error.downcast_ref::<wasmtime::Trap>().is_some() {
        RuntimeError::GuestTrap { export, message }
    } else {
        RuntimeError::GuestCall { export, message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knot_abi::{AbiVersion, SeverityPayload, SpanPayload};

    #[test]
    fn runs_rule_with_json_input_and_output() {
        let runtime = WasmRuntime::new();
        let wasm = fixture_rule_wasm(1);

        let metadata = runtime.metadata(&wasm).expect("metadata loads");
        assert_eq!(metadata.id, "knot/test-rule");

        let diagnostics = runtime
            .check(
                &wasm,
                &RuleInput {
                    facts: Vec::new(),
                    diagnostics: Vec::new(),
                },
            )
            .expect("rule runs");

        assert_eq!(
            diagnostics,
            vec![DiagnosticPayload {
                rule_id: "knot/test-rule".to_owned(),
                severity: SeverityPayload::Warning,
                message: "fixture diagnostic".to_owned(),
                span: SpanPayload {
                    file: "fixture.ts".to_owned(),
                    start_byte: 0,
                    end_byte: 9,
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 10,
                },
            }]
        );
    }

    #[test]
    fn rejects_abi_version_mismatch() {
        let runtime = WasmRuntime::new();
        let wasm = fixture_rule_wasm(2);

        assert!(matches!(
            runtime.metadata(&wasm),
            Err(RuntimeError::Abi(AbiError::VersionMismatch {
                expected,
                actual,
            })) if expected == knot_abi::ABI_VERSION && actual == AbiVersion::new(2)
        ));
        assert!(matches!(
            runtime.check(
                &wasm,
                &RuleInput {
                    facts: Vec::new(),
                    diagnostics: Vec::new(),
                },
            ),
            Err(RuntimeError::Abi(AbiError::VersionMismatch {
                expected,
                actual,
            })) if expected == knot_abi::ABI_VERSION && actual == AbiVersion::new(2)
        ));
    }

    #[test]
    fn reports_guest_traps_without_panicking() {
        let runtime = WasmRuntime::new();
        let wasm = fixture_rule_wasm_with_check("unreachable");

        assert!(matches!(
            runtime.check(&wasm, &empty_input()),
            Err(RuntimeError::GuestTrap {
                export: EXPORT_CHECK,
                ..
            })
        ));
    }

    #[test]
    fn stops_rules_that_exhaust_fuel() {
        let runtime = WasmRuntime::with_limits(RuntimeLimits {
            fuel: 10_000,
            ..RuntimeLimits::default()
        })
        .expect("runtime builds");
        let wasm = fixture_rule_wasm_with_check("(loop $forever (br $forever))");

        assert!(matches!(
            runtime.check(&wasm, &empty_input()),
            Err(RuntimeError::FuelExhausted {
                export: EXPORT_CHECK
            })
        ));
    }

    #[test]
    fn rejects_memory_growth_beyond_limit() {
        let runtime = WasmRuntime::with_limits(RuntimeLimits {
            memory_bytes: 64 * 1024,
            ..RuntimeLimits::default()
        })
        .expect("runtime builds");
        let wasm = fixture_rule_wasm_with_check("(drop (memory.grow (i32.const 1)))");

        let result = runtime.check(&wasm, &empty_input());
        assert!(
            matches!(
                result,
                Err(RuntimeError::MemoryLimitExceeded {
                    export: EXPORT_CHECK
                })
            ),
            "unexpected result: {result:?}"
        );
    }

    fn empty_input() -> RuleInput {
        RuleInput {
            facts: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn fixture_rule_wasm(abi_version: u32) -> Vec<u8> {
        let metadata = format!(
            r#"{{"abi_version":{abi_version},"id":"knot/test-rule","name":"Test Rule","severity":"warning"}}"#
        );
        let diagnostics = r#"[{"rule_id":"knot/test-rule","severity":"warning","message":"fixture diagnostic","span":{"file":"fixture.ts","start_byte":0,"end_byte":9,"start_line":1,"start_column":1,"end_line":1,"end_column":10}}]"#;
        let wat = format!(
            r#"
            (module
              (memory (export "memory") 1)
              (global $heap (mut i32) (i32.const 1024))
              (data (i32.const 16) "{metadata}")
              (data (i32.const 256) "{diagnostics}")

              (func $pack (param $ptr i32) (param $len i32) (result i64)
                (i64.or
                  (i64.shl (i64.extend_i32_u (local.get $ptr)) (i64.const 32))
                  (i64.extend_i32_u (local.get $len))))

              (func (export "knot_alloc") (param $len i32) (result i32)
                (local $ptr i32)
                (local.set $ptr (global.get $heap))
                (global.set $heap (i32.add (global.get $heap) (local.get $len)))
                (local.get $ptr))

              (func (export "knot_dealloc") (param $ptr i32) (param $len i32))

              (func (export "knot_metadata") (result i64)
                (call $pack (i32.const 16) (i32.const {metadata_len})))

              (func (export "knot_check") (param $ptr i32) (param $len i32) (result i64)
                (call $pack (i32.const 256) (i32.const {diagnostics_len}))))
            "#,
            metadata = wat_string(&metadata),
            diagnostics = wat_string(diagnostics),
            metadata_len = metadata.len(),
            diagnostics_len = diagnostics.len(),
        );
        wat::parse_str(wat).expect("fixture wat compiles")
    }

    fn fixture_rule_wasm_with_check(check_body: &str) -> Vec<u8> {
        let metadata =
            r#"{"abi_version":1,"id":"knot/test-rule","name":"Test Rule","severity":"warning"}"#;
        let wat = format!(
            r#"
            (module
              (memory (export "memory") 1)
              (data (i32.const 16) "{metadata}")

              (func (export "knot_alloc") (param $len i32) (result i32)
                (i32.const 1024))

              (func (export "knot_dealloc") (param $ptr i32) (param $len i32))

              (func (export "knot_metadata") (result i64)
                (i64.or
                  (i64.shl (i64.const 16) (i64.const 32))
                  (i64.const {metadata_len})))

              (func (export "knot_check") (param $ptr i32) (param $len i32) (result i64)
                {check_body}
                (i64.const 0)))
            "#,
            metadata = wat_string(metadata),
            metadata_len = metadata.len(),
        );
        wat::parse_str(wat).expect("fixture wat compiles")
    }

    fn wat_string(value: &str) -> String {
        value.replace('\\', "\\\\").replace('"', "\\\"")
    }
}
