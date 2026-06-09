use serde::Serialize;

pub fn render_diagnostics(diagnostics: &[knot_core::Diagnostic]) -> anyhow::Result<String> {
    let diagnostics: Vec<JsonDiagnostic> = diagnostics.iter().map(JsonDiagnostic::from).collect();
    Ok(serde_json::to_string_pretty(&diagnostics)?)
}

#[derive(Serialize)]
struct JsonDiagnostic {
    rule_id: String,
    severity: String,
    message: String,
    span: Option<JsonSpan>,
}

impl From<&knot_core::Diagnostic> for JsonDiagnostic {
    fn from(diagnostic: &knot_core::Diagnostic) -> Self {
        Self {
            rule_id: diagnostic.rule_id.to_string(),
            severity: diagnostic.severity.to_string(),
            message: diagnostic.message.to_string(),
            span: diagnostic.span.as_ref().map(JsonSpan::from),
        }
    }
}

#[derive(Serialize)]
struct JsonSpan {
    file: String,
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}

impl From<&knot_core::SourceSpan> for JsonSpan {
    fn from(span: &knot_core::SourceSpan) -> Self {
        Self {
            file: span.file.to_string(),
            start_byte: span.bytes.start,
            end_byte: span.bytes.end,
            start_line: span.start.line,
            start_column: span.start.column,
            end_line: span.end.line,
            end_column: span.end.column,
        }
    }
}
