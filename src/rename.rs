use std::collections::HashMap;

use tower_lsp::lsp_types::*;

use crate::definition;
use crate::document::Document;

pub fn rename(doc: &Document, pos: Position, new_name: String, uri: &Url) -> Option<WorkspaceEdit> {
    let refs = definition::find_references(doc, pos, uri);
    if refs.is_empty() {
        return None;
    }

    let edits: Vec<TextEdit> = refs
        .into_iter()
        .map(|loc| TextEdit {
            range: loc.range,
            new_text: new_name.clone(),
        })
        .collect();

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), edits);

    Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}

pub fn prepare_rename(doc: &Document, pos: Position) -> Option<PrepareRenameResponse> {
    let point = tree_sitter::Point {
        row: pos.line as usize,
        column: pos.character as usize,
    };
    let node = doc
        .tree
        .root_node()
        .named_descendant_for_point_range(point, point)?;

    if node.kind() != "var" {
        return None;
    }

    let range = Range {
        start: Position {
            line: node.start_position().row as u32,
            character: node.start_position().column as u32,
        },
        end: Position {
            line: node.end_position().row as u32,
            character: node.end_position().column as u32,
        },
    };

    Some(PrepareRenameResponse::Range(range))
}
