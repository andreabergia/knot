use std::fmt;

use knot_diagnostics::{
    ByteSpan, Diagnostic, DiagnosticMessage, FileId, LineColumn, RuleId, Severity, SourceSpan,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct AbiVersion(u32);

impl AbiVersion {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for AbiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub const ABI_VERSION: AbiVersion = AbiVersion::new(1);
pub const EXPORT_ALLOC: &str = "knot_alloc";
pub const EXPORT_DEALLOC: &str = "knot_dealloc";
pub const EXPORT_CHECK: &str = "knot_check";
pub const EXPORT_METADATA: &str = "knot_metadata";
pub const EXPORT_MEMORY: &str = "memory";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuleMetadata {
    pub abi_version: AbiVersion,
    pub id: String,
    pub name: String,
    pub severity: SeverityPayload,
}

impl RuleMetadata {
    pub fn validate_abi(&self) -> Result<(), AbiError> {
        if self.abi_version == ABI_VERSION {
            Ok(())
        } else {
            Err(AbiError::VersionMismatch {
                expected: ABI_VERSION,
                actual: self.abi_version,
            })
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "language", content = "fact")]
pub enum FactPayload {
    #[serde(rename = "python")]
    Python(PythonFactPayload),
    #[serde(rename = "typescript")]
    TypeScript(TypeScriptFactPayload),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PythonFactPayload {
    ParameterDefault {
        span: SpanPayload,
        function_name: String,
        parameter_name: String,
        literal: LiteralPayload,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TypeScriptFactPayload {
    Debugger {
        span: SpanPayload,
    },
    Call {
        span: SpanPayload,
        callee: String,
    },
    MemberAccess {
        span: SpanPayload,
        object: String,
        property: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuleInput {
    pub facts: Vec<FactPayload>,
    pub diagnostics: Vec<DiagnosticPayload>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticPayload {
    pub rule_id: String,
    pub severity: SeverityPayload,
    pub message: String,
    pub span: SpanPayload,
}

impl DiagnosticPayload {
    pub fn into_diagnostic(self) -> Diagnostic {
        let span = self.span.clone();
        Diagnostic::new(
            RuleId::new(self.rule_id),
            DiagnosticMessage::new(self.message),
            self.severity.into(),
        )
        .with_span(SourceSpan::new(
            FileId::new(span.file),
            ByteSpan::new(span.start_byte, span.end_byte),
            LineColumn::new(span.start_line, span.start_column),
            LineColumn::new(span.end_line, span.end_column),
        ))
    }
}

impl From<&Diagnostic> for DiagnosticPayload {
    fn from(diagnostic: &Diagnostic) -> Self {
        let span = diagnostic
            .span
            .as_ref()
            .expect("ABI diagnostics require source spans");
        Self {
            rule_id: diagnostic.rule_id.as_str().to_owned(),
            severity: diagnostic.severity.into(),
            message: diagnostic.message.as_str().to_owned(),
            span: SpanPayload {
                file: span.file.to_string(),
                start_byte: span.bytes.start,
                end_byte: span.bytes.end,
                start_line: span.start.line,
                start_column: span.start.column,
                end_line: span.end.line,
                end_column: span.end.column,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SpanPayload {
    pub file: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiteralPayload {
    List,
    Dict,
    Set,
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SeverityPayload {
    Error,
    Warning,
    Info,
    Hint,
}

impl From<SeverityPayload> for Severity {
    fn from(value: SeverityPayload) -> Self {
        match value {
            SeverityPayload::Error => Self::Error,
            SeverityPayload::Warning => Self::Warning,
            SeverityPayload::Info => Self::Info,
            SeverityPayload::Hint => Self::Hint,
        }
    }
}

impl From<Severity> for SeverityPayload {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Error => Self::Error,
            Severity::Warning => Self::Warning,
            Severity::Info => Self::Info,
            Severity::Hint => Self::Hint,
        }
    }
}

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum AbiError {
    #[error("unsupported ABI version: expected {expected}, got {actual}")]
    VersionMismatch {
        expected: AbiVersion,
        actual: AbiVersion,
    },
    #[error("invalid JSON payload: {0}")]
    Json(String),
}

pub fn encode_json<T: Serialize>(value: &T) -> Result<Vec<u8>, AbiError> {
    serde_json::to_vec(value).map_err(|error| AbiError::Json(error.to_string()))
}

pub fn decode_json<T>(bytes: &[u8]) -> Result<T, AbiError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_slice(bytes).map_err(|error| AbiError::Json(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_decodes_from_json() {
        let json = br#"{"abi_version":1,"id":"knot/ts-debugger","name":"No debugger","severity":"warning"}"#;

        let metadata: RuleMetadata = decode_json(json).expect("metadata decodes");

        assert_eq!(metadata.abi_version, ABI_VERSION);
        assert_eq!(metadata.id, "knot/ts-debugger");
        assert_eq!(metadata.severity, SeverityPayload::Warning);
    }

    #[test]
    fn diagnostic_decodes_from_json() {
        let json = br#"[{"rule_id":"knot/ts-debugger","severity":"warning","message":"Unexpected debugger statement.","span":{"file":"sample.ts","start_byte":0,"end_byte":8,"start_line":1,"start_column":1,"end_line":1,"end_column":9}}]"#;

        let diagnostics: Vec<DiagnosticPayload> = decode_json(json).expect("diagnostic decodes");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, "knot/ts-debugger");
        assert_eq!(diagnostics[0].span.end_column, 9);
    }

    #[test]
    fn language_specific_fact_decodes_from_json() {
        let json = br#"{"language":"python","fact":{"kind":"parameter_default","span":{"file":"sample.py","start_byte":12,"end_byte":14,"start_line":1,"start_column":13,"end_line":1,"end_column":15},"function_name":"f","parameter_name":"items","literal":"list"}}"#;

        let fact: FactPayload = decode_json(json).expect("fact decodes");

        assert!(matches!(
            fact,
            FactPayload::Python(PythonFactPayload::ParameterDefault {
                parameter_name,
                literal: LiteralPayload::List,
                ..
            }) if parameter_name == "items"
        ));
    }

    #[test]
    fn metadata_rejects_abi_version_mismatch() {
        let metadata = RuleMetadata {
            abi_version: AbiVersion::new(2),
            id: "knot/example".to_owned(),
            name: "Example".to_owned(),
            severity: SeverityPayload::Warning,
        };

        assert_eq!(
            metadata.validate_abi(),
            Err(AbiError::VersionMismatch {
                expected: ABI_VERSION,
                actual: AbiVersion::new(2),
            })
        );
    }
}
