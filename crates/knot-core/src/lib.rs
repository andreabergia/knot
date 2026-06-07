use std::{
    fs,
    path::{Path, PathBuf},
};

mod bundled_rules;

use knot_abi::{DiagnosticPayload, RuleInput};
pub use knot_diagnostics::{
    ByteSpan, Diagnostic, DiagnosticMessage, FileId, LineColumn, RuleId, Severity, SourceSpan,
    sort_diagnostics,
};
use knot_facts::extract_facts;
use knot_parser::{Language, SourceFile, parse_source};
use knot_runtime::WasmRuntime;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum CheckError {
    #[error("path does not exist: {0}")]
    MissingPath(PathBuf),
    #[error("failed to read directory {path}: {message}")]
    ReadDirectory { path: PathBuf, message: String },
    #[error("failed to inspect path {path}: {message}")]
    InspectPath { path: PathBuf, message: String },
    #[error("failed to read file {path}: {message}")]
    ReadFile { path: PathBuf, message: String },
    #[error("failed to parse file {path}: {source}")]
    ParseFile {
        path: PathBuf,
        source: knot_parser::ParseError,
    },
    #[error("bundled rule {rule_id} failed: {message}")]
    BundledRule { rule_id: String, message: String },
}

pub fn check_paths(paths: &[PathBuf]) -> Result<Vec<Diagnostic>, CheckError> {
    check_paths_with_rules(paths, bundled_rules::RULES)
}

fn check_paths_with_rules(
    paths: &[PathBuf],
    rules: &[bundled_rules::BundledRule],
) -> Result<Vec<Diagnostic>, CheckError> {
    for path in paths {
        if !path.exists() {
            return Err(CheckError::MissingPath(path.clone()));
        }
    }

    let mut diagnostics = Vec::new();
    let runtime = WasmRuntime::new();

    for path in paths {
        check_path(path, &runtime, rules, &mut diagnostics)?;
    }

    sort_diagnostics(&mut diagnostics);

    Ok(diagnostics)
}

fn check_path(
    path: &Path,
    runtime: &WasmRuntime,
    rules: &[bundled_rules::BundledRule],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), CheckError> {
    let metadata = fs::metadata(path).map_err(|error| CheckError::InspectPath {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    if metadata.is_dir() {
        check_directory(path, runtime, rules, diagnostics)
    } else if metadata.is_file() {
        check_file(path, runtime, rules, diagnostics)
    } else {
        Ok(())
    }
}

fn check_directory(
    path: &Path,
    runtime: &WasmRuntime,
    rules: &[bundled_rules::BundledRule],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), CheckError> {
    let entries = fs::read_dir(path).map_err(|error| CheckError::ReadDirectory {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let mut entry_paths = entries
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| CheckError::ReadDirectory {
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    entry_paths.sort();

    for entry_path in entry_paths {
        check_path(&entry_path, runtime, rules, diagnostics)?;
    }

    Ok(())
}

fn check_file(
    path: &Path,
    runtime: &WasmRuntime,
    rules: &[bundled_rules::BundledRule],
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<(), CheckError> {
    let Some(language) = Language::from_path(path) else {
        return Ok(());
    };

    let text = fs::read_to_string(path).map_err(|error| CheckError::ReadFile {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let source = SourceFile::new(FileId::new(path.to_path_buf()), text);
    let parsed = parse_source(&source, language).map_err(|source| CheckError::ParseFile {
        path: path.to_path_buf(),
        source,
    })?;

    let syntax_diagnostics = parsed.syntax_diagnostics(&source);
    let input = RuleInput {
        facts: extract_facts(&source, &parsed),
        diagnostics: syntax_diagnostics
            .iter()
            .map(DiagnosticPayload::from)
            .collect(),
    };
    diagnostics.extend(syntax_diagnostics);

    for rule in rules {
        let rule_diagnostics =
            runtime
                .check(rule.wasm, &input)
                .map_err(|error| CheckError::BundledRule {
                    rule_id: rule.id.to_owned(),
                    message: error.to_string(),
                })?;
        diagnostics.extend(
            rule_diagnostics
                .into_iter()
                .map(DiagnosticPayload::into_diagnostic),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn check_paths_accepts_existing_path() {
        let temp = TempFixture::new("existing-path");

        let diagnostics = check_paths(&[temp.path().to_path_buf()]).expect("path should exist");

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn check_paths_rejects_missing_path() {
        let error = check_paths(&[PathBuf::from("missing-knot-path")]).unwrap_err();

        assert_eq!(
            error,
            CheckError::MissingPath(PathBuf::from("missing-knot-path"))
        );
    }

    #[test]
    fn check_paths_reports_syntax_diagnostics_for_supported_files() {
        let temp = TempFixture::new("syntax-diagnostic");
        let source_path = temp.write_file("broken.py", "def broken(:\n    pass\n");

        let diagnostics = check_paths(std::slice::from_ref(&source_path)).expect("check succeeds");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, RuleId::new("knot/syntax"));
        assert_eq!(
            diagnostics[0].message,
            DiagnosticMessage::new("syntax error: missing )")
        );
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert_eq!(
            diagnostics[0].span,
            Some(SourceSpan::new(
                FileId::new(source_path),
                ByteSpan::new(11, 11),
                LineColumn::new(1, 12),
                LineColumn::new(1, 12),
            ))
        );
    }

    #[test]
    fn check_paths_runs_bundled_typescript_debugger_rule() {
        let temp = TempFixture::new("typescript-debugger");
        let source_path = temp.write_file("debugger.ts", "const answer = 42;\ndebugger;\n");

        let diagnostics = check_paths(std::slice::from_ref(&source_path)).expect("check succeeds");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, RuleId::new("knot/ts-debugger"));
        assert_eq!(
            diagnostics[0].message,
            DiagnosticMessage::new("Unexpected debugger statement.")
        );
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert_eq!(
            diagnostics[0].span,
            Some(SourceSpan::new(
                FileId::new(source_path),
                ByteSpan::new(19, 28),
                LineColumn::new(2, 1),
                LineColumn::new(2, 10),
            ))
        );
    }

    #[test]
    fn check_paths_accepts_an_empty_ruleset() {
        let temp = TempFixture::new("empty-ruleset");
        let source_path = temp.write_file("debugger.ts", "debugger;\n");

        let diagnostics = check_paths_with_rules(std::slice::from_ref(&source_path), &[])
            .expect("check succeeds");

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn check_paths_reports_malformed_bundled_rules() {
        let temp = TempFixture::new("malformed-rule");
        let source_path = temp.write_file("sample.ts", "const answer = 42;\n");
        let malformed_rule = bundled_rules::BundledRule {
            id: "knot/malformed",
            wasm: b"not wasm",
        };

        let error = check_paths_with_rules(&[source_path], &[malformed_rule]).unwrap_err();

        assert!(matches!(
            error,
            CheckError::BundledRule { rule_id, .. } if rule_id == "knot/malformed"
        ));
    }

    #[test]
    fn check_paths_ignores_unsupported_files() {
        let temp = TempFixture::new("unsupported-file");
        let source_path = temp.write_file("notes.txt", "def broken(:\n    pass\n");

        let diagnostics = check_paths(&[source_path]).expect("check succeeds");

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn check_paths_recurses_into_directories() {
        let temp = TempFixture::new("recursive-directory");
        temp.write_file("nested/broken.ts", "const answer = ;\n");

        let diagnostics = check_paths(&[temp.path().to_path_buf()]).expect("check succeeds");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, RuleId::new("knot/syntax"));
    }

    #[test]
    fn check_paths_sorts_diagnostics() {
        let temp = TempFixture::new("sorted-diagnostics");
        let later = temp.write_file("z.py", "def later(:\n    pass\n");
        let earlier = temp.write_file("a.py", "def earlier(:\n    pass\n");

        let diagnostics = check_paths(&[later, earlier.clone()]).expect("check succeeds");

        assert_eq!(diagnostics.len(), 2);
        assert_eq!(
            diagnostics[0].span.as_ref().map(|span| span.file.clone()),
            Some(FileId::new(earlier))
        );
    }

    struct TempFixture {
        path: PathBuf,
    }

    impl TempFixture {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("knot-core-{name}-{}-{unique}", std::process::id()));
            fs::create_dir(&path).expect("temp directory should be created");

            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn write_file(&self, relative_path: &str, contents: &str) -> PathBuf {
            let path = self.path.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("parent directories should be created");
            }
            fs::write(&path, contents).expect("fixture file should be written");
            path
        }
    }

    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
