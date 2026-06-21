use knot_sdk::{
    DiagnosticPayload, Rule, RuleContext, RuleMetadata, SeverityPayload, TypeScriptFactPayload,
    ABI_VERSION,
};

pub mod debugger {
    use super::*;

    #[derive(Default)]
    pub struct DebuggerRule;

    impl Rule for DebuggerRule {
        fn metadata() -> RuleMetadata {
            RuleMetadata {
                abi_version: ABI_VERSION,
                id: "knot/ts-debugger".to_owned(),
                name: "No debugger".to_owned(),
                severity: SeverityPayload::Warning,
            }
        }

        fn check(&self, ctx: &RuleContext) -> Vec<DiagnosticPayload> {
            ctx.typescript_facts()
                .filter_map(|fact| match fact {
                    TypeScriptFactPayload::Debugger { span } => Some(DiagnosticPayload::new(
                        "knot/ts-debugger",
                        SeverityPayload::Warning,
                        "Unexpected debugger statement.",
                        span.clone(),
                    )),
                    _ => None,
                })
                .collect()
        }
    }
}

pub mod console {
    use super::*;

    #[derive(Default)]
    pub struct ConsoleRule;

    impl Rule for ConsoleRule {
        fn metadata() -> RuleMetadata {
            RuleMetadata {
                abi_version: ABI_VERSION,
                id: "knot/ts-console".to_owned(),
                name: "No console".to_owned(),
                severity: SeverityPayload::Warning,
            }
        }

        fn check(&self, ctx: &RuleContext) -> Vec<DiagnosticPayload> {
            ctx.typescript_facts()
                .filter_map(|fact| match fact {
                    TypeScriptFactPayload::Call { span, callee }
                        if callee.starts_with("console.") =>
                    {
                        Some(DiagnosticPayload::new(
                            "knot/ts-console",
                            SeverityPayload::Warning,
                            "Unexpected console statement.",
                            span.clone(),
                        ))
                    }
                    _ => None,
                })
                .collect()
        }
    }
}