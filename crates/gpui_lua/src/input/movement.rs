use super::rope_ext::RopeExt;
use ropey::Rope;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MoveDirection {
    Up,
    Down,
}

pub fn prev_grapheme_boundary(text: &Rope, offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }
    let s = text.to_string();
    let clipped = text.clip_offset(offset, true);
    s.grapheme_indices(true)
        .rev()
        .find_map(|(idx, _)| (idx < clipped).then_some(idx))
        .unwrap_or(0)
}

pub fn next_grapheme_boundary(text: &Rope, offset: usize) -> usize {
    let max = text.len();
    if offset >= max {
        return max;
    }
    let s = text.to_string();
    s.grapheme_indices(true)
        .find_map(|(idx, _)| (idx > offset).then_some(idx))
        .unwrap_or(max)
}

pub fn prev_word_start(text: &Rope, offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }
    let s = text.to_string();
    let clipped = text.clip_offset(offset, true);
    let mut idx = prev_grapheme_boundary(text, clipped);
    // Skip whitespace
    while idx > 0 && s[idx..].chars().next().map_or(false, |c| c.is_whitespace()) {
        idx = prev_grapheme_boundary(text, idx);
    }
    // Skip word characters
    while idx > 0 && s[idx..].chars().next().map_or(false, |c| !c.is_whitespace()) {
        let prev = prev_grapheme_boundary(text, idx);
        if s[prev..].chars().next().map_or(false, |c| c.is_whitespace()) {
            break;
        }
        idx = prev;
    }
    idx
}

pub fn next_word_end(text: &Rope, offset: usize) -> usize {
    let max = text.len();
    if offset >= max {
        return max;
    }
    let s = text.to_string();
    let mut idx = next_grapheme_boundary(text, offset);
    // Skip word characters
    while idx < max && s[idx..].chars().next().map_or(false, |c| !c.is_whitespace()) {
        idx = next_grapheme_boundary(text, idx);
    }
    // Skip whitespace
    while idx < max && s[idx..].chars().next().map_or(false, |c| c.is_whitespace()) {
        idx = next_grapheme_boundary(text, idx);
    }
    idx
}
