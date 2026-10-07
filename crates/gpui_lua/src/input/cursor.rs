use gpui::Pixels;
use std::ops::{Range, RangeBounds};

/// Unique identifier for a cursor/selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct CursorId(pub usize);

impl CursorId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

/// A selection in the text, represented by start and end byte indices.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    pub start: usize,
    pub end: usize,
}

impl Selection {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn clear(&mut self) {
        self.start = 0;
        self.end = 0;
    }

    pub fn contains(&self, offset: usize) -> bool {
        offset >= self.start && offset < self.end
    }
}

impl From<Range<usize>> for Selection {
    fn from(value: Range<usize>) -> Self {
        Self::new(value.start, value.end)
    }
}

impl From<Selection> for Range<usize> {
    fn from(value: Selection) -> Self {
        value.start..value.end
    }
}

impl RangeBounds<usize> for Selection {
    fn start_bound(&self) -> std::ops::Bound<&usize> {
        std::ops::Bound::Included(&self.start)
    }

    fn end_bound(&self) -> std::ops::Bound<&usize> {
        std::ops::Bound::Excluded(&self.end)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct CursorSelection {
    pub id: CursorId,
    pub start: usize,
    pub end: usize,
    pub reversed: bool,
    pub column_anchor: Option<(Pixels, usize)>,
}

impl CursorSelection {
    pub fn new(id: CursorId, start: usize, end: usize) -> Self {
        Self {
            id,
            start,
            end,
            reversed: false,
            column_anchor: None,
        }
    }

    pub fn len(&self) -> usize {
        self.start.abs_diff(self.end)
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn clear(&mut self) {
        self.start = 0;
        self.end = 0;
        self.reversed = false;
    }

    pub fn contains(&self, offset: usize) -> bool {
        let (s, e) = if self.start <= self.end {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        };
        offset >= s && offset < e
    }

    pub fn cursor_offset(&self) -> usize {
        self.end
    }
    pub fn place_at(&mut self, offset: usize, column_anchor: Option<(Pixels, usize)>) {
        self.start = offset;
        self.end = offset;
        self.reversed = false;
        self.column_anchor = column_anchor;
    }

    pub fn is_collapsed(&self) -> bool {
        self.is_empty()
    }
}

impl From<Range<usize>> for CursorSelection {
    fn from(value: Range<usize>) -> Self {
        Self::new(CursorId::default(), value.start, value.end)
    }
}

impl From<CursorSelection> for Range<usize> {
    fn from(value: CursorSelection) -> Self {
        let (s, e) = if value.start <= value.end {
            (value.start, value.end)
        } else {
            (value.end, value.start)
        };
        s..e
    }
}

impl RangeBounds<usize> for CursorSelection {
    fn start_bound(&self) -> std::ops::Bound<&usize> {
        if self.start <= self.end {
            std::ops::Bound::Included(&self.start)
        } else {
            std::ops::Bound::Included(&self.end)
        }
    }

    fn end_bound(&self) -> std::ops::Bound<&usize> {
        if self.start <= self.end {
            std::ops::Bound::Excluded(&self.end)
        } else {
            std::ops::Bound::Excluded(&self.start)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Selections {
    selections: Vec<CursorSelection>,
    next_id: usize,
}

impl Default for Selections {
    fn default() -> Self {
        Self::new()
    }
}

impl Selections {
    pub fn new() -> Self {
        Self {
            selections: vec![CursorSelection::new(CursorId::new(0), 0, 0)],
            next_id: 1,
        }
    }

    pub fn active(&self) -> &CursorSelection {
        self.selections
            .first()
            .expect("Selections always has at least one selection")
    }

    pub fn active_mut(&mut self) -> &mut CursorSelection {
        self.selections
            .first_mut()
            .expect("Selections always has at least one selection")
    }

    pub fn iter(&self) -> impl Iterator<Item = &CursorSelection> {
        self.selections.iter()
    }

    pub fn len(&self) -> usize {
        self.selections.len()
    }

    pub fn is_single(&self) -> bool {
        self.selections.len() == 1
    }

    pub fn generate_id(&mut self) -> CursorId {
        let id = CursorId::new(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn add(&mut self, selection: CursorSelection) {
        self.selections.push(selection);
    }

    pub fn replace_all(&mut self, selections: Vec<CursorSelection>) {
        if !selections.is_empty() {
            self.selections = selections;
        }
    }

    pub fn remove_all_but_active(&mut self) {
        self.selections.truncate(1);
    }

    pub fn merge_overlapping(&mut self) {
        if self.selections.len() <= 1 {
            return;
        }

        let active_id = self.active().id;
        self.selections.sort_by_key(|s| s.start);

        let mut merged: Vec<CursorSelection> = Vec::with_capacity(self.selections.len());
        for selection in &self.selections {
            if let Some(last) = merged.last_mut() {
                if selection.start <= last.end {
                    last.end = last.end.max(selection.end);
                    if selection.id == active_id {
                        last.id = active_id;
                        last.reversed = selection.reversed;
                    }
                    continue;
                }
            }
            merged.push(*selection);
        }

        if let Some(active_pos) = merged.iter().position(|s| s.id == active_id) {
            let active = merged.remove(active_pos);
            merged.insert(0, active);
        }

        self.selections = merged;
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backward_selection_cursor_offset() {
        let mut sel = CursorSelection::new(CursorId::new(0), 10, 10);
        assert_eq!(sel.cursor_offset(), 10);

        // Select backwards from 10 to 4
        sel.end = 4;
        sel.reversed = true;

        assert_eq!(sel.cursor_offset(), 4, "Cursor must follow head to 4 when selecting backwards");
        assert_eq!(sel.len(), 6);
        assert!(sel.contains(5));
        assert!(!sel.contains(10));
        assert_eq!(Range::from(sel), 4..10);

        // Select forwards from 10 to 15
        sel.end = 15;
        sel.reversed = false;
        assert_eq!(sel.cursor_offset(), 15);
        assert_eq!(sel.len(), 5);
        assert_eq!(Range::from(sel), 10..15);
    }
}
