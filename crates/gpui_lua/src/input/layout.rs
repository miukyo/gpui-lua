use gpui::{Half, Pixels, ShapedLine, TextAlign, px};
use ropey::Rope;
use std::ops::Range;

use super::mask_pattern::MASK_CHAR;

#[derive(Clone, Default, Debug)]
pub struct LineLayout {
    pub shaped_line: ShapedLine,
    pub row: usize,
    pub byte_range: Range<usize>,
    pub width: Pixels,
    pub height: Pixels,
    pub is_masked: bool,
}

impl LineLayout {
    pub fn x_for_index(&self, text: &Rope, offset_in_line: usize) -> Pixels {
        let disp_offset = if self.is_masked {
            masked_display_offset(text, offset_in_line)
        } else {
            offset_in_line
        };
        self.shaped_line.x_for_index(disp_offset)
    }

    pub fn index_for_x(&self, text: &Rope, x: Pixels) -> usize {
        let disp_offset = self.shaped_line.index_for_x(x).unwrap_or(self.shaped_line.len());
        if self.is_masked {
            masked_to_original_offset(text, disp_offset)
        } else {
            disp_offset
        }
    }
}

pub fn masked_display_offset(text: &Rope, original_offset: usize) -> usize {
    let s = text.to_string();
    let original_offset = original_offset.min(s.len());
    let char_count = s[..original_offset].chars().count();
    char_count * MASK_CHAR.len_utf8()
}

pub fn masked_to_original_offset(text: &Rope, display_offset: usize) -> usize {
    let char_count = display_offset / MASK_CHAR.len_utf8();
    let s = text.to_string();
    let mut count = 0;
    for (i, _) in s.char_indices() {
        if count >= char_count {
            return i;
        }
        count += 1;
    }
    s.len()
}

#[derive(Clone, Default)]
pub struct LastLayout {
    pub lines: Vec<LineLayout>,
    pub visible_range: Range<usize>,
    pub content_size: gpui::Size<Pixels>,
    pub line_height: Pixels,
    pub text_align: TextAlign,
    pub space_width: Pixels,
    pub scroll_offset: gpui::Point<Pixels>,
}

impl LastLayout {
    pub fn line(&self, row: usize) -> Option<&LineLayout> {
        self.lines.get(row)
    }

    pub fn alignment_offset(&self, line_width: Pixels, container_width: Pixels) -> Pixels {
        match self.text_align {
            TextAlign::Left => px(0.0),
            TextAlign::Center => (container_width - line_width).half().max(px(0.0)),
            TextAlign::Right => (container_width - line_width).max(px(0.0)),
        }
    }
}
