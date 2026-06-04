pub use knot_diagnostics::FileId;
use knot_diagnostics::{ByteSpan, Diagnostic, LineColumn, Severity, SourceSpan};
use std::{error::Error, fmt, path::Path};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Language {
    Python,
    TypeScript,
    Tsx,
}

impl Language {
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("py") => Some(Self::Python),
            Some("ts") => Some(Self::TypeScript),
            Some("tsx") => Some(Self::Tsx),
            _ => None,
        }
    }
}

pub fn parse_source(source: &SourceFile, language: Language) -> Result<ParsedFile, ParseError> {
    let tree_sitter_language = match language {
        Language::Python => arborium::lang_python::language().into(),
        Language::TypeScript => arborium::lang_typescript::language().into(),
        Language::Tsx => arborium::lang_tsx::language().into(),
    };

    let mut parser = arborium::tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_language)
        .map_err(|_| ParseError::IncompatibleLanguage(language))?;
    let tree = parser
        .parse(source.text(), None)
        .ok_or(ParseError::ParseFailed(language))?;

    Ok(ParsedFile { language, tree })
}

#[derive(Debug)]
pub struct ParsedFile {
    language: Language,
    tree: arborium::tree_sitter::Tree,
}

impl ParsedFile {
    pub fn language(&self) -> Language {
        self.language
    }

    pub fn has_syntax_errors(&self) -> bool {
        self.tree.root_node().has_error()
    }

    pub fn syntax_diagnostics(&self, source: &SourceFile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        collect_syntax_diagnostics(self.tree.root_node(), source, &mut diagnostics);
        diagnostics
    }
}

fn collect_syntax_diagnostics(
    node: arborium::tree_sitter::Node<'_>,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if node.is_error() || node.is_missing() {
        let range = node.byte_range();
        let span = source.line_index().source_span(
            source.file_id().clone(),
            ByteSpan::new(range.start, range.end),
        );
        diagnostics.push(
            Diagnostic::new("knot/syntax", syntax_error_message(node), Severity::Error)
                .with_span(span),
        );
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.has_error() || child.is_error() || child.is_missing() {
            collect_syntax_diagnostics(child, source, diagnostics);
        }
    }
}

fn syntax_error_message(node: arborium::tree_sitter::Node<'_>) -> String {
    if node.is_missing() {
        format!("syntax error: missing {}", node.kind())
    } else {
        "syntax error: unexpected syntax".to_owned()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseError {
    IncompatibleLanguage(Language),
    ParseFailed(Language),
    UnsupportedLanguage(Language),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleLanguage(language) => {
                write!(f, "tree-sitter parser is incompatible with {language:?}")
            }
            Self::ParseFailed(language) => write!(f, "failed to parse {language:?} source"),
            Self::UnsupportedLanguage(language) => {
                write!(f, "no parser registered for {language:?}")
            }
        }
    }
}

impl Error for ParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    file_id: FileId,
    text: String,
    line_index: LineIndex,
}

impl SourceFile {
    pub fn new(file_id: FileId, text: impl Into<String>) -> Self {
        let text = text.into();
        let line_index = LineIndex::new(&text);

        Self {
            file_id,
            text,
            line_index,
        }
    }

    pub fn file_id(&self) -> &FileId {
        &self.file_id
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn line_index(&self) -> &LineIndex {
        &self.line_index
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineIndex {
    line_starts: Vec<usize>,
    char_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        let mut char_starts = Vec::new();

        for (offset, char) in text.char_indices() {
            char_starts.push(offset);

            if char == '\n' {
                line_starts.push(offset + 1);
            }
        }

        Self {
            line_starts,
            char_starts,
        }
    }

    pub fn line_column(&self, byte_offset: usize) -> LineColumn {
        let line_index = self
            .line_starts
            .partition_point(|start| *start <= byte_offset)
            - 1;
        let line_start = self.line_starts[line_index];
        let line_char_start = self
            .char_starts
            .partition_point(|start| *start < line_start);
        let offset_char_start = self
            .char_starts
            .partition_point(|start| *start < byte_offset);
        let column = offset_char_start - line_char_start + 1;

        LineColumn::new(line_index + 1, column)
    }

    pub fn source_span(&self, file: FileId, bytes: ByteSpan) -> SourceSpan {
        SourceSpan::new(
            file,
            bytes,
            self.line_column(bytes.start),
            self.line_column(bytes.end),
        )
    }
}

#[cfg(test)]
mod tests {
    use knot_diagnostics::{ByteSpan, LineColumn, Severity};

    use super::*;
    use std::path::Path;

    #[test]
    fn line_index_maps_ascii_byte_offsets_to_one_based_positions() {
        let source = SourceFile::new(FileId::new("sample.py"), "first\nsecond\n");

        assert_eq!(source.line_index().line_column(0), LineColumn::new(1, 1));
        assert_eq!(source.line_index().line_column(6), LineColumn::new(2, 1));
        assert_eq!(
            source.line_index().line_column(source.text().len()),
            LineColumn::new(3, 1)
        );
    }

    #[test]
    fn line_index_counts_utf8_columns_by_character() {
        let source = SourceFile::new(FileId::new("sample.py"), "éx\n");

        assert_eq!(source.line_index().line_column(0), LineColumn::new(1, 1));
        assert_eq!(
            source.line_index().line_column("é".len()),
            LineColumn::new(1, 2)
        );
        assert_eq!(
            source.line_index().line_column("éx\n".len()),
            LineColumn::new(2, 1)
        );
    }

    #[test]
    fn line_index_maps_multiline_byte_spans() {
        let source = SourceFile::new(FileId::new("sample.py"), "alpha\nbeta\ngamma\n");

        let span = source
            .line_index()
            .source_span(source.file_id().clone(), ByteSpan::new(2, 11));

        assert_eq!(span.start, LineColumn::new(1, 3));
        assert_eq!(span.end, LineColumn::new(3, 1));
    }

    #[test]
    fn language_detects_supported_file_extensions() {
        assert_eq!(
            Language::from_path(Path::new("sample.py")),
            Some(Language::Python)
        );
        assert_eq!(
            Language::from_path(Path::new("sample.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            Language::from_path(Path::new("component.tsx")),
            Some(Language::Tsx)
        );
    }

    #[test]
    fn language_rejects_unsupported_or_missing_extensions() {
        assert_eq!(Language::from_path(Path::new("README.md")), None);
        assert_eq!(Language::from_path(Path::new("Makefile")), None);
    }

    #[test]
    fn parse_source_parses_valid_python() {
        let source = SourceFile::new(FileId::new("sample.py"), "def answer():\n    return 42\n");

        let parsed = parse_source(&source, Language::Python).expect("python should parse");

        assert_eq!(parsed.language(), Language::Python);
        assert!(!parsed.has_syntax_errors());
    }

    #[test]
    fn parse_source_marks_python_syntax_errors() {
        let source = SourceFile::new(FileId::new("sample.py"), "def broken(:\n    pass\n");

        let parsed = parse_source(&source, Language::Python).expect("python should parse");

        assert!(parsed.has_syntax_errors());
    }

    #[test]
    fn parse_source_parses_valid_typescript() {
        let source = SourceFile::new(FileId::new("sample.ts"), "const answer = 42;\n");

        let parsed = parse_source(&source, Language::TypeScript).expect("typescript should parse");

        assert_eq!(parsed.language(), Language::TypeScript);
        assert!(!parsed.has_syntax_errors());
    }

    #[test]
    fn parse_source_marks_typescript_syntax_errors() {
        let source = SourceFile::new(FileId::new("sample.ts"), "const answer = ;\n");

        let parsed = parse_source(&source, Language::TypeScript).expect("typescript should parse");

        assert!(parsed.has_syntax_errors());
    }

    #[test]
    fn parse_source_parses_valid_tsx() {
        let source = SourceFile::new(
            FileId::new("component.tsx"),
            "export function Component() {\n    return <section>{42}</section>;\n}\n",
        );

        let parsed = parse_source(&source, Language::Tsx).expect("tsx should parse");

        assert_eq!(parsed.language(), Language::Tsx);
        assert!(!parsed.has_syntax_errors());
    }

    #[test]
    fn parse_source_marks_tsx_syntax_errors() {
        let source = SourceFile::new(
            FileId::new("component.tsx"),
            "export function Component() {\n    return <section>{42};\n}\n",
        );

        let parsed = parse_source(&source, Language::Tsx).expect("tsx should parse");

        assert!(parsed.has_syntax_errors());
    }

    #[test]
    fn syntax_diagnostics_report_parser_errors() {
        let source = SourceFile::new(FileId::new("sample.py"), "def broken(:\n    pass\n");
        let parsed = parse_source(&source, Language::Python).expect("python should parse");

        let diagnostics = parsed.syntax_diagnostics(&source);

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = diagnostics.first().expect("syntax diagnostic");
        assert_eq!(diagnostic.rule_id.as_str(), "knot/syntax");
        assert_eq!(diagnostic.message.as_str(), "syntax error: missing )");
        assert_eq!(diagnostic.severity, Severity::Error);
        let span = diagnostic.span.as_ref().expect("syntax diagnostic span");
        assert_eq!(span.file, FileId::new("sample.py"));
        assert_eq!(span.bytes, ByteSpan::new(11, 11));
        assert_eq!(span.start, LineColumn::new(1, 12));
        assert_eq!(span.end, LineColumn::new(1, 12));
    }

    #[test]
    fn syntax_diagnostics_count_utf8_columns_by_character() {
        let source = SourceFile::new(FileId::new("sample.py"), "é =\n");
        let parsed = parse_source(&source, Language::Python).expect("python should parse");

        let diagnostics = parsed.syntax_diagnostics(&source);

        let span = diagnostics
            .first()
            .and_then(|diagnostic| diagnostic.span.as_ref())
            .expect("syntax diagnostic should have a span");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].message.as_str(),
            "syntax error: unexpected syntax"
        );
        assert_eq!(span.file, FileId::new("sample.py"));
        assert_eq!(span.bytes, ByteSpan::new(0, 4));
        assert_eq!(span.start, LineColumn::new(1, 1));
        assert_eq!(span.end, LineColumn::new(1, 4));
    }

    #[test]
    fn syntax_diagnostics_can_span_multiple_lines() {
        let source = SourceFile::new(
            FileId::new("component.tsx"),
            "export function Component() {\n    return <section>\n        <span>{42}\n    </section>;\n}\n",
        );
        let parsed = parse_source(&source, Language::Tsx).expect("tsx should parse");

        let diagnostics = parsed.syntax_diagnostics(&source);

        assert_eq!(diagnostics.len(), 1);
        let span = diagnostics
            .first()
            .and_then(|diagnostic| diagnostic.span.as_ref())
            .expect("syntax diagnostic should have a span");
        assert_eq!(
            diagnostics[0].message.as_str(),
            "syntax error: unexpected syntax"
        );
        assert_eq!(span.file, FileId::new("component.tsx"));
        assert_eq!(span.bytes, ByteSpan::new(34, 86));
        assert_eq!(span.start, LineColumn::new(2, 5));
        assert_eq!(span.end, LineColumn::new(5, 1));
    }
}
