#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferId {
    Original,
    Added,
}

#[derive(Debug, Clone, Copy)]
pub struct Piece {
    pub buffer: BufferId,
    pub start: usize,
    pub length: usize,
}

/// A Piece Table implementation for efficient large-file text editing.
/// Keeps the original string immutable and appends edits to an `added` buffer.
#[derive(Debug, Clone)]
pub struct PieceTable {
    original: String,
    added: String,
    pieces: Vec<Piece>,
    total_length: usize,
}

impl PieceTable {
    /// Create a new Piece Table from an initial (original) string.
    pub fn new(text: String) -> Self {
        let length = text.len();
        let mut pt = Self {
            original: text,
            added: String::new(),
            pieces: Vec::new(),
            total_length: length,
        };
        if length > 0 {
            pt.pieces.push(Piece {
                buffer: BufferId::Original,
                start: 0,
                length,
            });
        }
        pt
    }

    /// Get the full text content.
    pub fn get_text(&self) -> String {
        let mut result = String::with_capacity(self.total_length);
        for piece in &self.pieces {
            match piece.buffer {
                BufferId::Original => {
                    result.push_str(&self.original[piece.start..piece.start + piece.length]);
                }
                BufferId::Added => {
                    result.push_str(&self.added[piece.start..piece.start + piece.length]);
                }
            }
        }
        result
    }

    /// Insert a string at the given offset.
    pub fn insert(&mut self, offset: usize, text: &str) {
        if text.is_empty() {
            return;
        }

        let add_start = self.added.len();
        let add_length = text.len();
        self.added.push_str(text);
        self.total_length += add_length;

        let new_piece = Piece {
            buffer: BufferId::Added,
            start: add_start,
            length: add_length,
        };

        if offset == 0 {
            self.pieces.insert(0, new_piece);
            return;
        }

        let mut current_offset = 0;
        for i in 0..self.pieces.len() {
            let piece = self.pieces[i];
            if current_offset + piece.length > offset {
                // Split the piece
                let split_point = offset - current_offset;

                let p1 = Piece {
                    buffer: piece.buffer,
                    start: piece.start,
                    length: split_point,
                };
                let p2 = Piece {
                    buffer: piece.buffer,
                    start: piece.start + split_point,
                    length: piece.length - split_point,
                };

                self.pieces[i] = p1;
                self.pieces.insert(i + 1, new_piece);
                if p2.length > 0 {
                    self.pieces.insert(i + 2, p2);
                }
                return;
            }
            current_offset += piece.length;
        }

        // If appended at the very end
        self.pieces.push(new_piece);
    }

    /// Delete a given length of characters starting at an offset.
    pub fn delete(&mut self, offset: usize, mut length: usize) {
        if length == 0 || offset >= self.total_length {
            return;
        }

        let mut current_offset = 0;
        let mut i = 0;

        while i < self.pieces.len() && length > 0 {
            let piece = self.pieces[i];

            if current_offset + piece.length > offset {
                // We found the start of the deletion
                let remove_start_in_piece = offset.saturating_sub(current_offset);
                let remove_length_in_piece =
                    std::cmp::min(length, piece.length - remove_start_in_piece);

                // If deleting from the middle, split the piece backwards
                if remove_start_in_piece > 0 {
                    let p1 = Piece {
                        buffer: piece.buffer,
                        start: piece.start,
                        length: remove_start_in_piece,
                    };

                    let remaining = piece.length - remove_start_in_piece - remove_length_in_piece;
                    self.pieces[i] = p1;

                    if remaining > 0 {
                        let p2 = Piece {
                            buffer: piece.buffer,
                            start: piece.start + remove_start_in_piece + remove_length_in_piece,
                            length: remaining,
                        };
                        self.pieces.insert(i + 1, p2);
                    }
                } else {
                    // Deleting from the very start of the piece
                    let remaining = piece.length - remove_length_in_piece;
                    if remaining > 0 {
                        self.pieces[i] = Piece {
                            buffer: piece.buffer,
                            start: piece.start + remove_length_in_piece,
                            length: remaining,
                        };
                    } else {
                        self.pieces.remove(i);
                        length -= remove_length_in_piece;
                        self.total_length -= remove_length_in_piece;
                        current_offset = offset;
                        continue;
                    }
                }

                length -= remove_length_in_piece;
                self.total_length -= remove_length_in_piece;

                current_offset = offset; // next pieces are deleted exactly at `offset`
            } else {
                current_offset += piece.length;
            }

            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piece_table() {
        let mut pt = PieceTable::new("Hello World!".to_string());

        pt.insert(5, " Beautiful");
        assert_eq!(pt.get_text(), "Hello Beautiful World!");

        pt.delete(6, 10);
        assert_eq!(pt.get_text(), "Hello World!");

        pt.insert(0, "Well, ");
        assert_eq!(pt.get_text(), "Well, Hello World!");

        pt.delete(0, 6);
        assert_eq!(pt.get_text(), "Hello World!");
    }
}
