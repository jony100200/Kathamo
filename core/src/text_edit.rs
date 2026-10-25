//! Editable text buffer with caret positioning, selection, and mutation primitives.
//!
//! Provides zero-allocation, robust UTF-8 text editing operations compliant with
//! desktop editor standards (Backspace, Delete, Arrow navigation, Home/End, Select All).

/// State and operations for an active single-line or multi-line text edit buffer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextEditBuffer {
    content: String,
    cursor: usize,
    selection_anchor: Option<usize>,
    focused: bool,
    revision: u64,
}

impl TextEditBuffer {
    /// Creates a new buffer initialized with the given text.
    pub fn new(initial: impl Into<String>) -> Self {
        let content = initial.into();
        let cursor = content.chars().count();
        Self {
            content,
            cursor,
            selection_anchor: None,
            focused: false,
            revision: 0,
        }
    }

    /// Returns the text content as a string slice.
    pub fn as_str(&self) -> &str {
        &self.content
    }

    /// Returns the character length of the content.
    pub fn char_len(&self) -> usize {
        self.content.chars().count()
    }

    /// Returns whether the buffer content is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Current character cursor index.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Changes only when content changes, so views can cache line indexes.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Place a character-indexed caret, preserving UTF-8 boundaries.
    pub fn set_cursor(&mut self, index: usize, extend_selection: bool) {
        if extend_selection {
            self.selection_anchor.get_or_insert(self.cursor);
        } else {
            self.selection_anchor = None;
        }
        self.cursor = index.min(self.char_len());
    }

    /// Whether the text edit buffer is currently focused.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Sets the focus state of the buffer.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.selection_anchor = None;
        }
    }

    /// Sets the content and places the cursor at the end.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.content = text.into();
        self.cursor = self.content.chars().count();
        self.selection_anchor = None;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Inserts a character at the current cursor position.
    pub fn insert_char(&mut self, c: char) {
        self.delete_selection();
        let byte_idx = self.cursor_byte_offset();
        self.content.insert(byte_idx, c);
        self.cursor += 1;
        self.selection_anchor = None;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Inserts a string at the current cursor position.
    pub fn insert_str(&mut self, text: &str) {
        self.delete_selection();
        let byte_idx = self.cursor_byte_offset();
        self.content.insert_str(byte_idx, text);
        self.cursor += text.chars().count();
        self.selection_anchor = None;
        self.revision = self.revision.wrapping_add(1);
    }

    /// Backspaces the character before the cursor (or deletes selection).
    pub fn backspace(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        if self.cursor > 0 {
            self.cursor -= 1;
            let byte_idx = self.cursor_byte_offset();
            self.content.remove(byte_idx);
            self.revision = self.revision.wrapping_add(1);
            true
        } else {
            false
        }
    }

    /// Deletes the character after the cursor (or deletes selection).
    pub fn delete(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        if self.cursor < self.char_len() {
            let byte_idx = self.cursor_byte_offset();
            self.content.remove(byte_idx);
            self.revision = self.revision.wrapping_add(1);
            true
        } else {
            false
        }
    }

    /// Moves the cursor one character to the left.
    pub fn move_left(&mut self, extend_selection: bool) {
        if !extend_selection {
            if let Some((start, _)) = self.selection_range() {
                self.cursor = start;
                self.selection_anchor = None;
                return;
            }
        }
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        } else if !extend_selection {
            self.selection_anchor = None;
        }
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Moves the cursor one character to the right.
    pub fn move_right(&mut self, extend_selection: bool) {
        if !extend_selection {
            if let Some((_, end)) = self.selection_range() {
                self.cursor = end;
                self.selection_anchor = None;
                return;
            }
        }
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        } else if !extend_selection {
            self.selection_anchor = None;
        }
        if self.cursor < self.char_len() {
            self.cursor += 1;
        }
    }

    /// Moves the cursor to the beginning of the text.
    pub fn move_home(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        } else if !extend_selection {
            self.selection_anchor = None;
        }
        self.cursor = 0;
    }

    /// Moves the cursor to the end of the text.
    pub fn move_end(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        } else if !extend_selection {
            self.selection_anchor = None;
        }
        self.cursor = self.char_len();
    }

    /// Selects all text in the buffer.
    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.char_len();
    }

    /// Returns the character range `(start, end)` of the active selection, if any.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.and_then(|anchor| {
            if anchor == self.cursor {
                None
            } else {
                Some((anchor.min(self.cursor), anchor.max(self.cursor)))
            }
        })
    }

    /// Returns the selected text, if any.
    pub fn selected_text(&self) -> Option<&str> {
        let (start, end) = self.selection_range()?;
        let start_byte = self.char_to_byte_offset(start);
        let end_byte = self.char_to_byte_offset(end);
        Some(&self.content[start_byte..end_byte])
    }

    /// Deletes the currently selected range, returning true if text was removed.
    pub fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection_range() {
            let start_byte = self.char_to_byte_offset(start);
            let end_byte = self.char_to_byte_offset(end);
            self.content.drain(start_byte..end_byte);
            self.revision = self.revision.wrapping_add(1);
            self.cursor = start;
            self.selection_anchor = None;
            true
        } else {
            false
        }
    }

    /// Clears the buffer.
    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
        self.selection_anchor = None;
        self.revision = self.revision.wrapping_add(1);
    }

    #[inline]
    fn cursor_byte_offset(&self) -> usize {
        self.char_to_byte_offset(self.cursor)
    }

    #[inline]
    fn char_to_byte_offset(&self, char_idx: usize) -> usize {
        self.content
            .char_indices()
            .nth(char_idx)
            .map(|(byte, _)| byte)
            .unwrap_or(self.content.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_edit_buffer_insert_and_backspace() {
        let mut buf = TextEditBuffer::new("Hello");
        assert_eq!(buf.as_str(), "Hello");
        assert_eq!(buf.cursor(), 5);

        buf.insert_str(" World");
        assert_eq!(buf.as_str(), "Hello World");

        buf.backspace();
        assert_eq!(buf.as_str(), "Hello Worl");

        buf.move_home(false);
        assert_eq!(buf.cursor(), 0);
        buf.insert_char('A');
        assert_eq!(buf.as_str(), "AHello Worl");
    }

    #[test]
    fn test_selection_and_deletion() {
        let mut buf = TextEditBuffer::new("Kathamo Studio");
        buf.select_all();
        assert_eq!(buf.selected_text(), Some("Kathamo Studio"));

        buf.insert_str("Rust");
        assert_eq!(buf.as_str(), "Rust");
        assert_eq!(buf.char_len(), 4);
    }

    #[test]
    fn test_bengali_unicode_char_offsets() {
        // Bengali characters have multi-byte UTF-8 representations
        let mut buf = TextEditBuffer::new("কাঠামো");
        assert_eq!(buf.char_len(), 6);
        buf.backspace();
        assert_eq!(buf.as_str(), "কাঠাম");
    }
}
