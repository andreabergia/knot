use arborium::tree_sitter::Node;
use knot_abi::SpanPayload;
use knot_diagnostics::ByteSpan;
use knot_parser::SourceFile;

pub(crate) fn span_payload(node: Node<'_>, source: &SourceFile) -> SpanPayload {
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

pub(crate) fn child_by_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    named_children(node)
        .into_iter()
        .find(|child| child.kind() == kind)
}

pub(crate) fn child_by_field_or_named_index<'tree>(
    node: Node<'tree>,
    field_name: &str,
    fallback_index: u32,
) -> Option<Node<'tree>> {
    node.child_by_field_name(field_name)
        .or_else(|| node.named_child(fallback_index))
}

pub(crate) fn child_text(node: Node<'_>, field_name: &str, source: &SourceFile) -> Option<String> {
    let child = node.child_by_field_name(field_name)?;
    node_text(child, source)
}

pub(crate) fn node_text(node: Node<'_>, source: &SourceFile) -> Option<String> {
    node.utf8_text(source.text().as_bytes())
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).filter(Node::is_named).collect()
}
