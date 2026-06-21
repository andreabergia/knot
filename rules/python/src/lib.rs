use knot_sdk::{
    DiagnosticPayload, LiteralPayload, PythonFactPayload, Rule, RuleContext, RuleMetadata,
    SeverityPayload, ABI_VERSION,
};

#[derive(Default)]
pub struct MutableDefaultArgRule;

impl Rule for MutableDefaultArgRule {
    fn metadata() -> RuleMetadata {
        RuleMetadata {
            abi_version: ABI_VERSION,
            id: "knot/py-mutable-default-arg".to_owned(),
            name: "No mutable defaults".to_owned(),
            severity: SeverityPayload::Warning,
        }
    }

    fn check(&self, ctx: &RuleContext) -> Vec<DiagnosticPayload> {
        ctx.python_facts()
            .filter_map(|fact| match fact {
                PythonFactPayload::ParameterDefault {
                    span, literal, ..
                } if is_mutable_literal(literal) => Some(DiagnosticPayload::new(
                    "knot/py-mutable-default-arg",
                    SeverityPayload::Warning,
                    "Mutable default argument.",
                    span.clone(),
                )),
                _ => None,
            })
            .collect()
    }
}

fn is_mutable_literal(literal: &LiteralPayload) -> bool {
    matches!(
        literal,
        LiteralPayload::List | LiteralPayload::Dict | LiteralPayload::Set
    )
}