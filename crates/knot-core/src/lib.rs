pub use knot_diagnostics::{
    ByteSpan, Diagnostic, FileId, LineColumn, Severity, SourceSpan, sort_diagnostics,
};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
