use std::fmt;

use knot_abi::FactPayload;
use knot_parser::{Language, ParsedFile, SourceFile};

mod python;
mod tree;
mod typescript;

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
        Language::Python => python::collect_facts(parsed.root_node(), source, &mut facts),
        Language::TypeScript | Language::Tsx => {
            typescript::collect_facts(parsed.root_node(), source, &mut facts)
        }
    }

    facts
}

#[cfg(test)]
mod tests {
    use knot_abi::{LiteralPayload, PythonFactPayload, SpanPayload, TypeScriptFactPayload};
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
