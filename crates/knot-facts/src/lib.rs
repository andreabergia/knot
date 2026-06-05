use std::fmt;

use arborium::tree_sitter::Node;
use knot_abi::{
    FactPayload, LiteralPayload, PythonFactPayload, SpanPayload, TypeScriptFactPayload,
};
use knot_diagnostics::ByteSpan;
use knot_parser::{Language, ParsedFile, SourceFile};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FactId(u64);

impl FactId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for FactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn extract_facts(source: &SourceFile, parsed: &ParsedFile) -> Vec<FactPayload> {
    let mut facts = Vec::new();

    match parsed.language() {
        Language::Python => collect_python_facts(parsed.root_node(), source, &mut facts),
        Language::TypeScript | Language::Tsx => {
            collect_typescript_facts(parsed.root_node(), source, &mut facts);
        }
    }

    facts
}

fn collect_python_facts(node: Node<'_>, source: &SourceFile, facts: &mut Vec<FactPayload>) {
    if node.kind() == "function_definition" {
        collect_python_function_parameter_defaults(node, source, facts);
    }

    for child in named_children(node) {
        collect_python_facts(child, source, facts);
    }
}

fn collect_python_function_parameter_defaults(
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

fn collect_typescript_facts(node: Node<'_>, source: &SourceFile, facts: &mut Vec<FactPayload>) {
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
        collect_typescript_facts(child, source, facts);
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

fn span_payload(node: Node<'_>, source: &SourceFile) -> SpanPayload {
    let bytes = node.byte_range();
    let span = source.line_index().source_span(
        source.file_id().clone(),
        ByteSpan::new(bytes.start, bytes.end),
    );

    SpanPayload {
        file: span.file.to_string(),
        start_byte: span.bytes.start,
        end_byte: span.bytes.end,
        start_line: span.start.line,
        start_column: span.start.column,
        end_line: span.end.line,
        end_column: span.end.column,
    }
}

fn child_by_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    named_children(node)
        .into_iter()
        .find(|child| child.kind() == kind)
}

fn child_by_field_or_named_index<'tree>(
    node: Node<'tree>,
    field_name: &str,
    fallback_index: u32,
) -> Option<Node<'tree>> {
    node.child_by_field_name(field_name)
        .or_else(|| node.named_child(fallback_index))
}

fn child_text(node: Node<'_>, field_name: &str, source: &SourceFile) -> Option<String> {
    let child = node.child_by_field_name(field_name)?;
    node_text(child, source)
}

fn node_text(node: Node<'_>, source: &SourceFile) -> Option<String> {
    node.utf8_text(source.text().as_bytes())
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).filter(Node::is_named).collect()
}

#[cfg(test)]
mod tests {
    use knot_diagnostics::FileId;
    use knot_parser::{Language, SourceFile, parse_source};

    use super::*;

    #[test]
    fn extracts_python_parameter_default_literals() {
        let source = SourceFile::new(
            FileId::new("sample.py"),
            "def f(items=[], opts={}, seen=set(), count=0):\n    pass\n",
        );
        let parsed = parse_source(&source, Language::Python).expect("source parses");

        let facts = extract_facts(&source, &parsed);

        assert_eq!(
            facts,
            vec![
                FactPayload::Python(PythonFactPayload::ParameterDefault {
                    span: span("sample.py", 6, 14, 1, 7, 1, 15),
                    function_name: "f".to_owned(),
                    parameter_name: "items".to_owned(),
                    literal: LiteralPayload::List,
                }),
                FactPayload::Python(PythonFactPayload::ParameterDefault {
                    span: span("sample.py", 16, 23, 1, 17, 1, 24),
                    function_name: "f".to_owned(),
                    parameter_name: "opts".to_owned(),
                    literal: LiteralPayload::Dict,
                }),
                FactPayload::Python(PythonFactPayload::ParameterDefault {
                    span: span("sample.py", 25, 35, 1, 26, 1, 36),
                    function_name: "f".to_owned(),
                    parameter_name: "seen".to_owned(),
                    literal: LiteralPayload::Set,
                }),
                FactPayload::Python(PythonFactPayload::ParameterDefault {
                    span: span("sample.py", 37, 44, 1, 38, 1, 45),
                    function_name: "f".to_owned(),
                    parameter_name: "count".to_owned(),
                    literal: LiteralPayload::Other,
                }),
            ]
        );
    }

    #[test]
    fn extracts_typescript_debugger_calls_and_member_accesses() {
        let source = SourceFile::new(
            FileId::new("sample.ts"),
            "debugger;\nconsole.log(\"x\");\nfoo.bar();\n",
        );
        let parsed = parse_source(&source, Language::TypeScript).expect("source parses");

        let facts = extract_facts(&source, &parsed);

        assert_eq!(
            facts,
            vec![
                FactPayload::TypeScript(TypeScriptFactPayload::Debugger {
                    span: span("sample.ts", 0, 9, 1, 1, 1, 10),
                }),
                FactPayload::TypeScript(TypeScriptFactPayload::Call {
                    span: span("sample.ts", 10, 26, 2, 1, 2, 17),
                    callee: "console.log".to_owned(),
                }),
                FactPayload::TypeScript(TypeScriptFactPayload::MemberAccess {
                    span: span("sample.ts", 10, 21, 2, 1, 2, 12),
                    object: "console".to_owned(),
                    property: "log".to_owned(),
                }),
                FactPayload::TypeScript(TypeScriptFactPayload::Call {
                    span: span("sample.ts", 28, 37, 3, 1, 3, 10),
                    callee: "foo.bar".to_owned(),
                }),
                FactPayload::TypeScript(TypeScriptFactPayload::MemberAccess {
                    span: span("sample.ts", 28, 35, 3, 1, 3, 8),
                    object: "foo".to_owned(),
                    property: "bar".to_owned(),
                }),
            ]
        );
    }

    fn span(
        file: &str,
        start: usize,
        end: usize,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> SpanPayload {
        SpanPayload {
            file: file.to_owned(),
            start_byte: start,
            end_byte: end,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }
}
