use super::rope_ext::RopeExt;
use super::text_boundary::word_range_from_chars;
use ropey::Rope;
use std::ops::Range;

pub struct TextSelector;

impl TextSelector {
    /// Select a line in the given text at the specified byte offset.
    pub fn line_range(text: &Rope, offset: usize) -> Range<usize> {
        let offset = text.clip_offset(offset, true);
        let point = text.offset_to_point(offset);
        let start = text.line_start_offset(point.row);
        let end = text.line_end_offset(point.row);
        start..end
    }

    /// Select a word in the given text at the specified byte offset.
    pub fn word_range(text: &Rope, offset: usize) -> Option<Range<usize>> {
        let offset = text.clip_offset(offset, true);
        let char = text.char_at(offset)?;
        let end = offset + char.len_utf8();

        let s = text.to_string();
        let prev_chars = s[..offset].chars().rev().take(128);
        let next_chars = s[end..].chars().take(128);

        Some(word_range_from_chars(offset, char, prev_chars, next_chars))
    }
}
