use std::error::Error;
use std::fmt;
use std::path::PathBuf;

pub use knot_diagnostics::{
    ByteSpan, Diagnostic, FileId, LineColumn, Severity, SourceSpan, sort_diagnostics,
};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Eq, PartialEq)]
pub enum CheckError {
    MissingPath(PathBuf),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPath(path) => write!(f, "path does not exist: {}", path.display()),
        }
    }
}

impl Error for CheckError {}

pub fn check_paths(paths: &[PathBuf]) -> Result<Vec<Diagnostic>, CheckError> {
    for path in paths {
        if !path.exists() {
            return Err(CheckError::MissingPath(path.clone()));
        }
    }

    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_paths_accepts_existing_path() {
        let diagnostics = check_paths(&[PathBuf::from(".")]).expect("path should exist");

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
}
