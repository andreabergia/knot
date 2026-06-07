use arborium::tree_sitter::Node;
use knot_abi::{FactPayload, TypeScriptFactPayload};
use knot_parser::SourceFile;

use crate::tree::{child_text, named_children, span_payload};

pub(crate) fn collect_facts(node: Node<'_>, source: &SourceFile, facts: &mut Vec<FactPayload>) {
    match node.kind() {
        "debugger_statement" => {
            facts.push(FactPayload::TypeScript(TypeScriptFactPayload::Debugger {
                span: span_payload(node, source),
            }))
        }
        "call_expression" => {
            if let Some(callee) = child_text(node, "function", source) {
                facts.push(FactPayload::TypeScript(TypeScriptFactPayload::Call {
                    span: span_payload(node, source),
                    callee,
                }));
            }
        }
        "member_expression" => {
            if let (Some(object), Some(property)) = (
                child_text(node, "object", source),
                child_text(node, "property", source),
            ) {
                facts.push(FactPayload::TypeScript(
                    TypeScriptFactPayload::MemberAccess {
                        span: span_payload(node, source),
                        object,
                        property,
                    },
                ));
            }
        }
        _ => {}
    }

    for child in named_children(node) {
        collect_facts(child, source, facts);
    }
}
