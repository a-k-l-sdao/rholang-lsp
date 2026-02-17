use tree_sitter::{InputEdit, Parser, Point, Tree};

pub struct Document {
    pub source: String,
    pub tree: Tree,
}

impl Document {
    pub fn new(parser: &mut Parser, source: String) -> Option<Self> {
        let tree = parser.parse(&source, None)?;
        Some(Document { source, tree })
    }

    /// Apply incremental edits and reparse.
    pub fn apply_change(
        &mut self,
        parser: &mut Parser,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
        new_text: &str,
    ) {
        let start_byte = self.offset_at(start_line, start_col);
        let old_end_byte = self.offset_at(end_line, end_col);

        // Apply the text replacement
        self.source.replace_range(start_byte..old_end_byte, new_text);

        // Compute new end position
        let new_end_byte = start_byte + new_text.len();
        let (new_end_line, new_end_col) = self.position_at(new_end_byte);

        let edit = InputEdit {
            start_byte,
            old_end_byte,
            new_end_byte,
            start_position: Point {
                row: start_line,
                column: start_col,
            },
            old_end_position: Point {
                row: end_line,
                column: end_col,
            },
            new_end_position: Point {
                row: new_end_line,
                column: new_end_col,
            },
        };

        self.tree.edit(&edit);

        if let Some(new_tree) = parser.parse(&self.source, Some(&self.tree)) {
            self.tree = new_tree;
        }
    }

    /// Full reparse (for full-sync mode or when incremental gets confused).
    pub fn reparse(&mut self, parser: &mut Parser, source: String) {
        self.source = source;
        if let Some(new_tree) = parser.parse(&self.source, None) {
            self.tree = new_tree;
        }
    }

    fn offset_at(&self, line: usize, col: usize) -> usize {
        let mut offset = 0;
        for (i, l) in self.source.lines().enumerate() {
            if i == line {
                return offset + col.min(l.len());
            }
            offset += l.len() + 1; // +1 for newline
        }
        self.source.len()
    }

    fn position_at(&self, byte_offset: usize) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        for (i, ch) in self.source.char_indices() {
            if i >= byte_offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += ch.len_utf8();
            }
        }
        (line, col)
    }
}
