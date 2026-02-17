use tower_lsp::lsp_types::*;
use tree_sitter::Node;

use crate::diagnostics::node_range;
use crate::document::Document;

#[allow(deprecated)] // DocumentSymbol.deprecated field
pub fn document_symbols(doc: &Document) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    collect_symbols(doc.tree.root_node(), &doc.source, &mut symbols);
    symbols
}

#[allow(deprecated)]
fn collect_symbols(node: Node, source: &str, symbols: &mut Vec<DocumentSymbol>) {
    match node.kind() {
        "contract" => {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = name_node.utf8_text(source.as_bytes()).unwrap_or("?");
                let range = node_range(node);
                let sel = node_range(name_node);

                // Collect children symbols inside the contract body
                let mut children = Vec::new();
                if let Some(body) = node.child_by_field_name("proc") {
                    collect_symbols(body, source, &mut children);
                }

                symbols.push(DocumentSymbol {
                    name: name.to_string(),
                    detail: Some("contract".into()),
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    deprecated: None,
                    range,
                    selection_range: sel,
                    children: if children.is_empty() {
                        None
                    } else {
                        Some(children)
                    },
                });
                return; // Don't recurse again — we already handled children
            }
        }
        "new" => {
            if let Some(decls) = node.child_by_field_name("decls") {
                let mut cursor = decls.walk();
                for decl in decls.children_by_field_name("decl", &mut cursor) {
                    if decl.kind() == "name_decl" {
                        if let Some(var_node) = decl.child(0) {
                            if var_node.kind() == "var" {
                                let name =
                                    var_node.utf8_text(source.as_bytes()).unwrap_or("?");
                                symbols.push(DocumentSymbol {
                                    name: name.to_string(),
                                    detail: Some("channel".into()),
                                    kind: SymbolKind::VARIABLE,
                                    tags: None,
                                    deprecated: None,
                                    range: node_range(decl),
                                    selection_range: node_range(var_node),
                                    children: None,
                                });
                            }
                        }
                    }
                }
            }
            // Also collect symbols from the body
            if let Some(proc) = node.child_by_field_name("proc") {
                collect_symbols(proc, source, symbols);
            }
            return;
        }
        _ => {}
    }

    // Default: recurse into all children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_symbols(child, source, symbols);
    }
}
