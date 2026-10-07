use ropey::{LineType, Rope, RopeSlice};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point {
    pub row: usize,
    pub column: usize,
}

impl Point {
    pub fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }
}

pub trait RopeExt {
    fn line_start_offset(&self, row: usize) -> usize;
    fn line_end_offset(&self, row: usize) -> usize;
    fn slice_line(&self, row: usize) -> RopeSlice<'_>;
    fn lines_len(&self) -> usize;
    fn line_len(&self, row: usize) -> usize;
    fn offset_to_point(&self, offset: usize) -> Point;
    fn point_to_offset(&self, point: Point) -> usize;
    fn clip_offset(&self, offset: usize, bias_left: bool) -> usize;
    fn char_at(&self, offset: usize) -> Option<char>;
    fn replace_range(&mut self, range: Range<usize>, new_text: &str);
    fn offset_to_utf16(&self, offset: usize) -> usize;
    fn offset_from_utf16(&self, offset_utf16: usize) -> usize;
}

impl RopeExt for Rope {
    fn slice_line(&self, row: usize) -> RopeSlice<'_> {
        let total_lines = self.lines_len();
        if row >= total_lines {
            return self.slice(0..0);
        }

        let line = self.line(row, LineType::LF);
        if line.len() > 0 {
            let line_end = line.len() - 1;
            if line.is_char_boundary(line_end) && line.char(line_end) == '\n' {
                return line.slice(..line_end);
            }
        }
        line
    }

    fn lines_len(&self) -> usize {
        self.len_lines(LineType::LF)
    }

    fn line_len(&self, row: usize) -> usize {
        self.slice_line(row).len()
    }

    fn line_start_offset(&self, row: usize) -> usize {
        self.point_to_offset(Point::new(row, 0))
    }

    fn line_end_offset(&self, row: usize) -> usize {
        if row >= self.lines_len() {
            return self.len();
        }
        self.line_start_offset(row) + self.line_len(row)
    }

    fn offset_to_point(&self, offset: usize) -> Point {
        let offset = self.clip_offset(offset, true);
        let row = self.byte_to_line_idx(offset, LineType::LF);
        let line_start = self.line_to_byte_idx(row, LineType::LF);
        let column = offset.saturating_sub(line_start);
        Point::new(row, column)
    }

    fn point_to_offset(&self, point: Point) -> usize {
        if point.row >= self.lines_len() {
            return self.len();
        }
        let line_start = self.line_to_byte_idx(point.row, LineType::LF);
        line_start + point.column
    }

    fn clip_offset(&self, offset: usize, bias_left: bool) -> usize {
        if offset > self.len() {
            return self.len();
        }
        if self.is_char_boundary(offset) {
            return offset;
        }
        if bias_left {
            self.floor_char_boundary(offset)
        } else {
            self.ceil_char_boundary(offset)
        }
    }

    fn char_at(&self, offset: usize) -> Option<char> {
        if offset >= self.len() {
            return None;
        }
        self.get_char(offset).ok()
    }

    fn replace_range(&mut self, range: Range<usize>, new_text: &str) {
        let start = self.clip_offset(range.start, true);
        let end = self.clip_offset(range.end, false);
        let clamped_range = start..end.max(start);
        self.remove(clamped_range.clone());
        self.insert(clamped_range.start, new_text);
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let offset = self.clip_offset(offset, true);
        self.byte_to_utf16_idx(offset)
    }

    fn offset_from_utf16(&self, offset_utf16: usize) -> usize {
        if offset_utf16 > self.len_utf16() {
            return self.len();
        }
        self.utf16_to_byte_idx(offset_utf16)
    }
}
