use tower_lsp::lsp_types::*;
use tree_sitter::Point;

use crate::document::Document;

pub fn hover(doc: &Document, pos: Position) -> Option<Hover> {
    let point = Point {
        row: pos.line as usize,
        column: pos.character as usize,
    };

    let node = doc
        .tree
        .root_node()
        .named_descendant_for_point_range(point, point)?;

    let source = doc.source.as_bytes();
    let text = node.utf8_text(source).unwrap_or("");
    let kind = node.kind();

    // Build context based on parent
    let context = match node.parent().map(|p| p.kind()) {
        Some("contract") => {
            let parent = node.parent().unwrap();
            if parent
                .child_by_field_name("name")
                .map(|n| n.id() == node.id())
                .unwrap_or(false)
            {
                "contract name".to_string()
            } else {
                "in contract".to_string()
            }
        }
        Some("name_decl") => "channel declaration (new)".to_string(),
        Some("names") => {
            // Could be contract formals or bind names
            if let Some(gp) = node.parent().and_then(|p| p.parent()) {
                match gp.kind() {
                    "contract" => "contract parameter".to_string(),
                    "linear_bind" | "repeated_bind" | "peek_bind" => {
                        "bound name".to_string()
                    }
                    _ => format!("name in {}", gp.kind()),
                }
            } else {
                "name".to_string()
            }
        }
        Some("send") => "send target".to_string(),
        Some("eval") => "evaluated name".to_string(),
        Some("method") => {
            let parent = node.parent().unwrap();
            if parent
                .child_by_field_name("name")
                .map(|n| n.id() == node.id())
                .unwrap_or(false)
            {
                "method name".to_string()
            } else {
                "method target".to_string()
            }
        }
        _ => kind.to_string(),
    };

    // Check for preceding comment
    let comment = find_preceding_comment(node, source);

    let mut parts = Vec::new();
    parts.push(format!("```rholang\n{text}\n```"));
    parts.push(format!("**{context}** (`{kind}`)"));
    if let Some(c) = comment {
        parts.push(format!("---\n{c}"));
    }

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: parts.join("\n\n"),
        }),
        range: Some(Range {
            start: Position {
                line: node.start_position().row as u32,
                character: node.start_position().column as u32,
            },
            end: Position {
                line: node.end_position().row as u32,
                character: node.end_position().column as u32,
            },
        }),
    })
}

fn find_preceding_comment(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut prev = node;
    // Walk up to a statement-level node
    while let Some(parent) = prev.parent() {
        if parent.kind() == "source_file" || parent.kind() == "block" {
            break;
        }
        prev = parent;
    }

    // Check the sibling before this statement
    if let Some(sib) = prev.prev_sibling() {
        if sib.kind() == "line_comment" || sib.kind() == "block_comment" {
            let text = sib.utf8_text(source).unwrap_or("");
            let cleaned: String = text
                .lines()
                .map(|l| {
                    l.trim()
                        .trim_start_matches("//")
                        .trim_start_matches("/*")
                        .trim_end_matches("*/")
                        .trim()
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Some(cleaned);
        }
    }
    None
}
