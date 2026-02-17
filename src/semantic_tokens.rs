use tower_lsp::lsp_types::*;
use tree_sitter::Node;

use crate::document::Document;

// Token type indices — must match LEGEND_TYPE order
const TT_KEYWORD: u32 = 0;
const TT_VARIABLE: u32 = 1;
const TT_FUNCTION: u32 = 2;
const TT_STRING: u32 = 3;
const TT_NUMBER: u32 = 4;
const TT_OPERATOR: u32 = 5;
const TT_COMMENT: u32 = 6;
const TT_TYPE: u32 = 7;
const TT_PARAMETER: u32 = 8;
const TT_METHOD: u32 = 9;

pub const LEGEND_TYPE: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::STRING,
    SemanticTokenType::NUMBER,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::COMMENT,
    SemanticTokenType::TYPE,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::METHOD,
];

pub fn semantic_tokens(doc: &Document) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    collect_tokens(doc.tree.root_node(), &doc.source, &mut tokens);

    // Sort by position
    tokens.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    // Convert to delta-encoded
    let mut result = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for (line, col, len, token_type) in tokens {
        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 {
            col - prev_start
        } else {
            col
        };
        result.push(SemanticToken {
            delta_line,
            delta_start,
            length: len,
            token_type,
            token_modifiers_bitset: 0,
        });
        prev_line = line;
        prev_start = col;
    }

    result
}

// Collect (line, col, len, token_type) tuples
fn collect_tokens(node: Node, source: &str, tokens: &mut Vec<(u32, u32, u32, u32)>) {
    let kind = node.kind();
    let start = node.start_position();
    let end = node.end_position();
    let line = start.row as u32;
    let col = start.column as u32;

    // Single-line token length; for multiline tokens use the full byte range
    let len = if start.row == end.row {
        (end.column - start.column) as u32
    } else {
        node.byte_range().len() as u32
    };

    match kind {
        // Keywords
        "new" | "in" | "contract" | "for" | "select" | "match" | "if" | "else" | "let"
        | "not" | "and" | "or" | "matches" | "bundle_write" | "bundle_read"
        | "bundle_equiv" | "bundle_read_write" => {
            tokens.push((line, col, len, TT_KEYWORD));
        }

        // Comments
        "line_comment" | "block_comment" => {
            // For multiline comments, emit one token per line
            if start.row == end.row {
                tokens.push((line, col, len, TT_COMMENT));
            } else {
                let text = node.utf8_text(source.as_bytes()).unwrap_or("");
                let mut current_line = start.row as u32;
                let mut current_col = start.column as u32;
                for line_text in text.lines() {
                    tokens.push((
                        current_line,
                        current_col,
                        line_text.len() as u32,
                        TT_COMMENT,
                    ));
                    current_line += 1;
                    current_col = 0;
                }
            }
        }

        // Literals
        "string_literal" | "uri_literal" => {
            tokens.push((line, col, len, TT_STRING));
        }
        "long_literal" => {
            tokens.push((line, col, len, TT_NUMBER));
        }
        "bool_literal" | "nil" => {
            tokens.push((line, col, len, TT_KEYWORD));
        }

        // Types
        "simple_type" => {
            tokens.push((line, col, len, TT_TYPE));
        }

        // Var — classify based on parent
        "var" => {
            let parent_kind = node.parent().map(|p| p.kind());
            let token_type = match parent_kind {
                Some("contract") => {
                    // Is this the contract name?
                    let parent = node.parent().unwrap();
                    if parent
                        .child_by_field_name("name")
                        .map(|n| n.id() == node.id())
                        .unwrap_or(false)
                    {
                        TT_FUNCTION
                    } else {
                        TT_VARIABLE
                    }
                }
                Some("method") => {
                    let parent = node.parent().unwrap();
                    if parent
                        .child_by_field_name("name")
                        .map(|n| n.id() == node.id())
                        .unwrap_or(false)
                    {
                        TT_METHOD
                    } else {
                        TT_VARIABLE
                    }
                }
                Some("name_decl") => TT_PARAMETER,
                Some("names") => {
                    // Check grandparent to distinguish formals from other names
                    if let Some(gp) = node.parent().and_then(|p| p.parent()) {
                        match gp.kind() {
                            "contract" => TT_PARAMETER,
                            _ => TT_VARIABLE,
                        }
                    } else {
                        TT_VARIABLE
                    }
                }
                _ => TT_VARIABLE,
            };
            tokens.push((line, col, len, token_type));
        }

        // Don't emit tokens for structural nodes; recurse into children
        _ => {
            // Check if this is a keyword-like anonymous node
            if !node.is_named() {
                // Anonymous nodes like operators, punctuation, keywords
                match kind {
                    "!" | "!!" | "!?" | "<-" | "<=" | "<<-" | "=>" | "+" | "-" | "*" | "/"
                    | "%" | "++" | "--" | "%%" | "==" | "!=" | "<" | ">" | ">=" | "\\/"
                    | "/\\" | "~" | "=" | "|" | "&" => {
                        tokens.push((line, col, len, TT_OPERATOR));
                    }
                    _ => {}
                }
            }
        }
    }

    // Recurse into named children for structural nodes
    if node.is_named()
        && !matches!(
            kind,
            "string_literal" | "uri_literal" | "long_literal" | "line_comment" | "block_comment"
        )
    {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect_tokens(child, source, tokens);
        }
    }
}
