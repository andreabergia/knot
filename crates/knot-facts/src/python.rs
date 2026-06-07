use arborium::tree_sitter::Node;
use knot_abi::{FactPayload, LiteralPayload, PythonFactPayload};
use knot_parser::SourceFile;

use crate::tree::{
    child_by_field_or_named_index, child_by_kind, child_text, named_children, span_payload,
};

pub(crate) fn collect_facts(node: Node<'_>, source: &SourceFile, facts: &mut Vec<FactPayload>) {
    if node.kind() == "function_definition" {
        collect_function_parameter_defaults(node, source, facts);
    }

    for child in named_children(node) {
        collect_facts(child, source, facts);
    }
}

fn collect_function_parameter_defaults(
    function: Node<'_>,
    source: &SourceFile,
    facts: &mut Vec<FactPayload>,
) {
    let Some(function_name) = child_text(function, "name", source) else {
        return;
    };
    let Some(parameters) = child_by_kind(function, "parameters") else {
        return;
    };

    for parameter in named_children(parameters) {
        if parameter.kind() != "default_parameter" {
            continue;
        }

        let Some(parameter_name) = child_text(parameter, "name", source) else {
            continue;
        };
        let Some(default_value) = child_by_field_or_named_index(parameter, "value", 1) else {
            continue;
        };

        facts.push(FactPayload::Python(PythonFactPayload::ParameterDefault {
            span: span_payload(parameter, source),
            function_name: function_name.clone(),
            parameter_name,
            literal: literal_payload(default_value, source),
        }));
    }
}

fn literal_payload(node: Node<'_>, source: &SourceFile) -> LiteralPayload {
    match node.kind() {
        "list" => LiteralPayload::List,
        "dictionary" => LiteralPayload::Dict,
        "set" => LiteralPayload::Set,
        "call" if child_text(node, "function", source).as_deref() == Some("set") => {
            LiteralPayload::Set
        }
        _ => LiteralPayload::Other,
    }
}
