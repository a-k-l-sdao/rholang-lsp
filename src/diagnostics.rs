use tower_lsp::lsp_types::*;
use tree_sitter::Node;

use crate::document::Document;

pub fn collect_diagnostics(doc: &Document) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    collect_errors(doc.tree.root_node(), &doc.source, &mut diagnostics);
    diagnostics
}

fn collect_errors(node: Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
    if node.is_error() {
        let range = node_range(node);
        let text = node
            .utf8_text(source.as_bytes())
            .unwrap_or("")
            .chars()
            .take(40)
            .collect::<String>();
        diagnostics.push(Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("rholang-lsp".into()),
            message: format!("Syntax error near `{text}`"),
            ..Default::default()
        });
    } else if node.is_missing() {
        let range = node_range(node);
        let kind = node.kind();
        diagnostics.push(Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("rholang-lsp".into()),
            message: format!("Missing `{kind}`"),
            ..Default::default()
        });
    } else {
        // Only recurse into children if this node might contain errors
        if node.has_error() {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_errors(child, source, diagnostics);
            }
        }
    }
}

pub fn node_range(node: Node) -> Range {
    let start = node.start_position();
    let end = node.end_position();
    Range {
        start: Position {
            line: start.row as u32,
            character: start.column as u32,
        },
        end: Position {
            line: end.row as u32,
            character: end.column as u32,
        },
    }
}
