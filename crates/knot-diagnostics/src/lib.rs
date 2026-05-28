use std::cmp::Ordering;
use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FileId(PathBuf);

impl FileId {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RuleId(String);

impl RuleId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for RuleId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RuleId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DiagnosticMessage(String);

impl DiagnosticMessage {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiagnosticMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for DiagnosticMessage {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for DiagnosticMessage {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ByteSpan {
    pub start: usize,
    pub end: usize,
}

impl ByteSpan {
    pub fn new(start: usize, end: usize) -> Self {
        assert!(start <= end, "span start must be before span end");
        Self { start, end }
    }

    pub fn len(self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

impl LineColumn {
    pub fn new(line: usize, column: usize) -> Self {
        assert!(line > 0, "line numbers are 1-based");
        assert!(column > 0, "column numbers are 1-based");
        Self { line, column }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceSpan {
    pub file: FileId,
    pub bytes: ByteSpan,
    pub start: LineColumn,
    pub end: LineColumn,
}

impl SourceSpan {
    pub fn new(file: FileId, bytes: ByteSpan, start: LineColumn, end: LineColumn) -> Self {
        Self {
            file,
            bytes,
            start,
            end,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Diagnostic {
    pub rule_id: RuleId,
    pub message: DiagnosticMessage,
    pub severity: Severity,
    pub span: Option<SourceSpan>,
}

impl Diagnostic {
    pub fn new(
        rule_id: impl Into<RuleId>,
        message: impl Into<DiagnosticMessage>,
        severity: Severity,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            message: message.into(),
            severity,
            span: None,
        }
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }
}

impl Ord for Diagnostic {
    fn cmp(&self, other: &Self) -> Ordering {
        self.span
            .cmp(&other.span)
            .then_with(|| self.rule_id.cmp(&other.rule_id))
            .then_with(|| self.severity.cmp(&other.severity))
            .then_with(|| self.message.cmp(&other.message))
    }
}

impl PartialOrd for Diagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_span_reports_length() {
        let span = ByteSpan::new(2, 7);

        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());
    }

    #[test]
    fn diagnostics_sort_by_span_then_rule() {
        let file = FileId::new("sample.py");
        let later = SourceSpan::new(
            file.clone(),
            ByteSpan::new(10, 12),
            LineColumn::new(2, 1),
            LineColumn::new(2, 3),
        );
        let earlier = SourceSpan::new(
            file,
            ByteSpan::new(0, 1),
            LineColumn::new(1, 1),
            LineColumn::new(1, 2),
        );
        let mut diagnostics = vec![
            Diagnostic::new("z-rule", "later", Severity::Warning).with_span(later),
            Diagnostic::new("a-rule", "earlier", Severity::Warning).with_span(earlier),
        ];

        sort_diagnostics(&mut diagnostics);

        assert_eq!(diagnostics[0].rule_id, RuleId::new("a-rule"));
        assert_eq!(diagnostics[1].rule_id, RuleId::new("z-rule"));
    }

    #[test]
    fn severity_formats_as_lowercase() {
        assert_eq!(Severity::Warning.to_string(), "warning");
    }
}
