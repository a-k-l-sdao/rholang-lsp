use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Point};

use crate::document::Document;

const LOCALS_QUERY: &str = include_str!("../queries/locals.scm");

/// Resolve goto-definition for the symbol at the given position.
pub fn goto_definition(doc: &Document, pos: Position) -> Option<Location> {
    let point = Point {
        row: pos.line as usize,
        column: pos.character as usize,
    };
    let root = doc.tree.root_node();
    let cursor_node = root.named_descendant_for_point_range(point, point)?;

    // Only resolve var nodes
    if cursor_node.kind() != "var" {
        return None;
    }

    let name = cursor_node.utf8_text(doc.source.as_bytes()).ok()?;

    // Find the definition of this name in the nearest enclosing scope
    find_definition(doc, cursor_node, name)
}

/// Find all references to the symbol at the given position.
pub fn find_references(doc: &Document, pos: Position, uri: &Url) -> Vec<Location> {
    let point = Point {
        row: pos.line as usize,
        column: pos.character as usize,
    };
    let root = doc.tree.root_node();
    let cursor_node = match root.named_descendant_for_point_range(point, point) {
        Some(n) => n,
        None => return vec![],
    };

    if cursor_node.kind() != "var" {
        return vec![];
    }

    let name = match cursor_node.utf8_text(doc.source.as_bytes()) {
        Ok(n) => n,
        Err(_) => return vec![],
    };

    // First, find the definition site
    let def_node = match find_definition_node(doc, cursor_node, name) {
        Some(n) => n,
        None => cursor_node, // if we're on the definition itself
    };

    // Find the scope that contains this definition
    let scope = find_enclosing_scope(def_node);

    // Collect all var nodes with the same name within this scope
    let mut refs = Vec::new();
    collect_var_refs(scope, name, doc.source.as_bytes(), uri, &mut refs);
    refs
}

fn find_definition(doc: &Document, cursor_node: Node, name: &str) -> Option<Location> {
    let def_node = find_definition_node(doc, cursor_node, name)?;
    Some(Location {
        uri: Url::parse("file:///").unwrap(), // placeholder — caller replaces
        range: Range {
            start: Position {
                line: def_node.start_position().row as u32,
                character: def_node.start_position().column as u32,
            },
            end: Position {
                line: def_node.end_position().row as u32,
                character: def_node.end_position().column as u32,
            },
        },
    })
}

fn find_definition_node<'a>(doc: &'a Document, cursor_node: Node<'a>, name: &str) -> Option<Node<'a>> {
    let source = doc.source.as_bytes();

    // Walk up through scopes looking for a definition of this name
    let mut scope_node = cursor_node;
    loop {
        let parent = scope_node.parent()?;

        match parent.kind() {
            "new" => {
                // Check name_decls
                if let Some(decls) = parent.child_by_field_name("decls") {
                    if let Some(found) = find_var_in_subtree(decls, name, source) {
                        return Some(found);
                    }
                }
            }
            "contract" => {
                // Check contract name
                if let Some(name_node) = parent.child_by_field_name("name") {
                    if name_node.utf8_text(source).ok() == Some(name) {
                        return Some(name_node);
                    }
                }
                // Check formals
                if let Some(formals) = parent.child_by_field_name("formals") {
                    if let Some(found) = find_var_in_subtree(formals, name, source) {
                        return Some(found);
                    }
                }
            }
            "input" => {
                // Check receipts for bound names
                if let Some(receipts) = parent.child_by_field_name("receipts") {
                    if let Some(found) = find_var_in_names(receipts, name, source) {
                        return Some(found);
                    }
                }
            }
            "let" => {
                if let Some(decls) = parent.child_by_field_name("decls") {
                    if let Some(found) = find_var_in_subtree(decls, name, source) {
                        return Some(found);
                    }
                }
            }
            "case" | "branch" => {
                if let Some(pattern) = parent.child_by_field_name("pattern") {
                    if let Some(found) = find_var_in_subtree(pattern, name, source) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }

        scope_node = parent;
    }
}

/// Find a var node with the given name anywhere in a subtree.
fn find_var_in_subtree<'a>(node: Node<'a>, name: &str, source: &[u8]) -> Option<Node<'a>> {
    if node.kind() == "var" && node.utf8_text(source).ok() == Some(name) {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_var_in_subtree(child, name, source) {
            return Some(found);
        }
    }
    None
}

/// Find a var inside names/linear_bind/repeated_bind subtrees.
fn find_var_in_names<'a>(node: Node<'a>, name: &str, source: &[u8]) -> Option<Node<'a>> {
    find_var_in_subtree(node, name, source)
}

fn find_enclosing_scope(node: Node) -> Node {
    let mut current = node;
    loop {
        match current.kind() {
            "source_file" | "block" => return current,
            _ => match current.parent() {
                Some(p) => current = p,
                None => return current,
            },
        }
    }
}

fn collect_var_refs(node: Node, name: &str, source: &[u8], uri: &Url, refs: &mut Vec<Location>) {
    if node.kind() == "var" && node.utf8_text(source).ok() == Some(name) {
        refs.push(Location {
            uri: uri.clone(),
            range: Range {
                start: Position {
                    line: node.start_position().row as u32,
                    character: node.start_position().column as u32,
                },
                end: Position {
                    line: node.end_position().row as u32,
                    character: node.end_position().column as u32,
                },
            },
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_var_refs(child, name, source, uri, refs);
    }
}
