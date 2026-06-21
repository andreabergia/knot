use knot_abi::RuleInput;

pub use knot_abi::{
    ABI_VERSION, AbiVersion, DiagnosticPayload, FactPayload, LiteralPayload, PythonFactPayload,
    RuleMetadata, SeverityPayload, SpanPayload, TypeScriptFactPayload,
};

pub trait Rule: Default {
    fn metadata() -> RuleMetadata;

    fn check(&self, ctx: &RuleContext) -> Vec<DiagnosticPayload>;
}

pub struct RuleContext<'a> {
    input: &'a RuleInput,
}

impl<'a> RuleContext<'a> {
    pub fn facts(&self) -> &[FactPayload] {
        &self.input.facts
    }

    pub fn python_facts(&self) -> impl Iterator<Item = &PythonFactPayload> {
        self.input.facts.iter().filter_map(|f| match f {
            FactPayload::Python(p) => Some(p),
            _ => None,
        })
    }

    pub fn typescript_facts(&self) -> impl Iterator<Item = &TypeScriptFactPayload> {
        self.input.facts.iter().filter_map(|f| match f {
            FactPayload::TypeScript(t) => Some(t),
            _ => None,
        })
    }

    pub fn syntax_diagnostics(&self) -> &[DiagnosticPayload] {
        &self.input.diagnostics
    }
}

/// Internal — construct a `RuleContext` from a `RuleInput` reference.
/// Public so `run_check` can use it.
impl<'a> From<&'a RuleInput> for RuleContext<'a> {
    fn from(input: &'a RuleInput) -> Self {
        Self { input }
    }
}

pub fn encode_json<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, knot_abi::AbiError> {
    knot_abi::encode_json(value)
}

pub fn decode_json<T>(bytes: &[u8]) -> Result<T, knot_abi::AbiError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    knot_abi::decode_json(bytes)
}

pub fn run_check<R: Rule>(input_bytes: &[u8]) -> Vec<u8> {
    let input: RuleInput = decode_json(input_bytes).expect("invalid check input JSON");
    let ctx = RuleContext::from(&input);
    let rule = R::default();
    let diagnostics = rule.check(&ctx);
    encode_json(&diagnostics).expect("failed to encode check output")
}

#[macro_export]
macro_rules! register {
    ($ty:ty) => {
        static mut HEAP: [u8; 1024 * 1024] = [0; 1024 * 1024];
        static mut HEAP_CURSOR: usize = 0;

        #[unsafe(no_mangle)]
        pub extern "C" fn knot_alloc(len: u32) -> u32 {
            // The host creates a fresh instance per `metadata()`/`check()`
            // call, so the bump cursor resets implicitly. If instance reuse
            // is ever introduced, this arena will exhaust after ~1 MiB of
            // cumulative allocations and must be reset between calls.
            let offset;
            unsafe {
                offset = HEAP_CURSOR;
                let next = offset
                    .checked_add(len as usize)
                    .expect("knot_alloc: overflow");
                if next > HEAP.len() {
                    panic!("knot_alloc: out of memory");
                }
                HEAP_CURSOR = next;
            }
            let base = unsafe { HEAP.as_ptr() as usize };
            (base + offset) as u32
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn knot_dealloc(_ptr: u32, _len: u32) {}

        #[unsafe(no_mangle)]
        pub extern "C" fn knot_metadata() -> u64 {
            let bytes = $crate::encode_json(&<$ty as $crate::Rule>::metadata())
                .expect("metadata serialization failed");
            let ptr = knot_alloc(bytes.len() as u32);
            unsafe {
                ::core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
            }
            ((ptr as u64) << 32) | (bytes.len() as u64)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn knot_check(ptr: u32, len: u32) -> u64 {
            let input_slice =
                unsafe { ::core::slice::from_raw_parts(ptr as *const u8, len as usize) };
            let output = $crate::run_check::<$ty>(input_slice);
            let out_ptr = knot_alloc(output.len() as u32);
            unsafe {
                ::core::ptr::copy_nonoverlapping(output.as_ptr(), out_ptr as *mut u8, output.len());
            }
            ((out_ptr as u64) << 32) | (output.len() as u64)
        }

        fn main() {}
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_context_iterates_facts_by_language() {
        let input = RuleInput {
            facts: vec![
                FactPayload::Python(PythonFactPayload::ParameterDefault {
                    span: SpanPayload {
                        file: "test.py".to_owned(),
                        start_byte: 0,
                        end_byte: 1,
                        start_line: 1,
                        start_column: 1,
                        end_line: 1,
                        end_column: 2,
                    },
                    function_name: "f".to_owned(),
                    parameter_name: "x".to_owned(),
                    literal: LiteralPayload::List,
                }),
                FactPayload::TypeScript(TypeScriptFactPayload::Debugger {
                    span: SpanPayload {
                        file: "test.ts".to_owned(),
                        start_byte: 0,
                        end_byte: 1,
                        start_line: 1,
                        start_column: 1,
                        end_line: 1,
                        end_column: 2,
                    },
                }),
            ],
            diagnostics: vec![DiagnosticPayload::new(
                "knot/syntax",
                SeverityPayload::Error,
                "parse error",
                SpanPayload {
                    file: "test.ts".to_owned(),
                    start_byte: 0,
                    end_byte: 1,
                    start_line: 1,
                    start_column: 1,
                    end_line: 1,
                    end_column: 2,
                },
            )],
        };
        let ctx = RuleContext::from(&input);

        assert_eq!(ctx.python_facts().count(), 1);
        assert_eq!(ctx.typescript_facts().count(), 1);
        assert_eq!(ctx.syntax_diagnostics().len(), 1);
    }

    #[derive(Default)]
    struct FakeRule;

    impl Rule for FakeRule {
        fn metadata() -> RuleMetadata {
            RuleMetadata {
                abi_version: ABI_VERSION,
                id: "knot/test".to_owned(),
                name: "Test Rule".to_owned(),
                severity: SeverityPayload::Warning,
            }
        }

        fn check(&self, _ctx: &RuleContext) -> Vec<DiagnosticPayload> {
            vec![DiagnosticPayload::new(
                "knot/test",
                SeverityPayload::Warning,
                "test diagnostic",
                SpanPayload {
                    file: "test.ts".to_owned(),
                    start_byte: 1,
                    end_byte: 5,
                    start_line: 1,
                    start_column: 2,
                    end_line: 1,
                    end_column: 6,
                },
            )]
        }
    }

    #[test]
    fn run_check_roundtips_json() {
        let input = RuleInput {
            facts: Vec::new(),
            diagnostics: Vec::new(),
        };
        let input_bytes = encode_json(&input).expect("serialize");

        let output_bytes = run_check::<FakeRule>(&input_bytes);
        let diagnostics: Vec<DiagnosticPayload> = decode_json(&output_bytes).expect("deserialize");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "knot/test");
        assert_eq!(diagnostics[0].message, "test diagnostic");
        assert_eq!(diagnostics[0].severity, SeverityPayload::Warning);
    }
}
