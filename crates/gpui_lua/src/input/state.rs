use super::blink_cursor::BlinkCursor;
use super::change::Change;
use super::cursor::{CursorId, CursorSelection, Selections};
use super::kind::InputModeKind;
use super::layout::LastLayout;
use super::movement::{next_grapheme_boundary, next_word_end, prev_grapheme_boundary, prev_word_start};
use super::rope_ext::RopeExt;
use super::selection::TextSelector;
use super::undo_manager::{EditIntent, UndoManager};
use crate::dsl::convert::LuaInvoker;
use crate::dsl::node::SyncRegistryKey;
use gpui::{
    AppContext, Bounds, ClipboardItem, Context, DefiniteLength, Entity, EntityInputHandler,
    FocusHandle, Hsla, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Point, ScrollHandle, ScrollWheelEvent, UTF16Selection, Window, px,
};
use ropey::Rope;
use std::ops::Range;
use std::sync::Arc;

pub struct InputBaseState<M: InputModeKind> {
    pub text: Rope,
    pub selections: Selections,
    pub undo_manager: UndoManager,
    pub blink_cursor: Entity<BlinkCursor>,
    pub focus_handle: FocusHandle,
    pub masked: bool,
    pub placeholder: String,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub required: bool,
    pub disabled: bool,
    pub read_only: bool,
    pub is_valid: bool,
    pub error_msg: Option<String>,
    pub selecting: bool,
    pub ime_marked_range: Option<CursorSelection>,
    pub scroll_handle: ScrollHandle,
    pub last_layout: Option<LastLayout>,
    pub last_bounds: Option<Bounds<Pixels>>,
    pub rows: usize,
    pub max_rows: Option<usize>,
    pub last_external_value: Option<String>,
    _marker: std::marker::PhantomData<M>,

    // Visual appearance styling
    pub bg: Option<Hsla>,
    pub focus_bg: Option<Hsla>,
    pub border_color: Option<Hsla>,
    pub focus_border_color: Option<Hsla>,
    pub error_border_color: Option<Hsla>,
    pub border_width: f32,
    pub rounded: f32,
    pub px: f32,
    pub py: f32,
    pub font_size: f32,
    pub font_family: Option<String>,
    pub text_color: Option<Hsla>,
    pub placeholder_color: Option<Hsla>,
    pub cursor_color: Option<Hsla>,
    pub selection_bg: Option<Hsla>,
    pub width: Option<DefiniteLength>,
    pub height: Option<DefiniteLength>,

    // Callbacks
    pub on_change: Option<SyncRegistryKey>,
    pub on_input: Option<SyncRegistryKey>,
    pub on_submit: Option<SyncRegistryKey>,
    pub on_focus: Option<SyncRegistryKey>,
    pub on_blur: Option<SyncRegistryKey>,
    pub on_validate: Option<SyncRegistryKey>,
    pub invoker: Option<Arc<dyn LuaInvoker>>,
}

impl<M: InputModeKind> InputBaseState<M> {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let blink_cursor = cx.new(|_| BlinkCursor::new());

        let blink_entity = blink_cursor.clone();
        let _ = cx.on_focus(&focus_handle, window, move |_this, _window, cx| {
            blink_entity.update(cx, |b, cx| b.start(cx));
        });

        let blink_entity_blur = blink_cursor.clone();
        let _ = cx.on_blur(&focus_handle, window, move |this, _window, cx| {
            blink_entity_blur.update(cx, |b, cx| b.stop(cx));
            let cur = this.cursor();
            this.selections.active_mut().place_at(cur, None);
            this.selecting = false;
            this.trigger_blur();
            cx.notify();
        });

        Self {
            text: Rope::new(),
            selections: Selections::new(),
            undo_manager: UndoManager::new(),
            blink_cursor,
            focus_handle,
            masked: false,
            placeholder: String::from("Enter text..."),
            max_length: None,
            pattern: None,
            required: false,
            disabled: false,
            read_only: false,
            is_valid: true,
            error_msg: None,
            selecting: false,
            ime_marked_range: None,
            scroll_handle: ScrollHandle::new(),
            last_layout: None,
            last_bounds: None,
            rows: if M::MULTI_LINE { 4 } else { 1 },
            max_rows: None,
            last_external_value: None,
            _marker: std::marker::PhantomData,

            bg: None,
            focus_bg: None,
            border_color: None,
            focus_border_color: None,
            error_border_color: None,
            border_width: 1.0,
            rounded: 6.0,
            px: 12.0,
            py: 8.0,
            font_size: 14.0,
            font_family: None,
            text_color: None,
            placeholder_color: None,
            cursor_color: None,
            selection_bg: None,
            width: None,
            height: None,

            on_change: None,
            on_input: None,
            on_submit: None,
            on_focus: None,
            on_blur: None,
            on_validate: None,
            invoker: None,
        }
    }

    pub fn cursor(&self) -> usize {
        let len = self.text.len();
        let cur = self.selections.active().cursor_offset().min(len);
        self.text.clip_offset(cur, true)
    }

    pub fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if self.text.to_string() != text {
            self.text = Rope::from(text);
            let end = self.text.len();
            let cur = self.cursor().min(end);
            self.selections.active_mut().place_at(cur, None);
            self.validate();
            cx.notify();
        }
    }

    pub fn text(&self) -> String {
        self.text.to_string()
    }

    pub fn is_multi_line(&self) -> bool {
        M::MULTI_LINE
    }

    pub fn pause_blink_cursor(&self, cx: &mut Context<Self>) {
        self.blink_cursor.update(cx, |b, cx| b.pause(cx));
    }

    pub fn has_selection(&self) -> bool {
        !self.selections.active().is_collapsed()
    }

    pub fn selected_range(&self) -> Range<usize> {
        let len = self.text.len();
        let active = self.selections.active();
        let (s, e) = if active.start <= active.end {
            (active.start, active.end)
        } else {
            (active.end, active.start)
        };
        let s = self.text.clip_offset(s.min(len), true);
        let e = self.text.clip_offset(e.min(len), false);
        s..e.max(s)
    }

    pub fn delete_selection(&mut self, cx: &mut Context<Self>) -> bool {
        if self.has_selection() {
            let range = self.selected_range();
            let old_text = self.text.slice(range.clone()).to_string();
            self.undo_manager.record_transaction(
                Change::new(range.start..range.end, &old_text, range.start..range.start, ""),
                EditIntent::Backspace,
            );
            self.text.replace_range(range.clone(), "");
            self.selections.active_mut().place_at(range.start, None);
            self.pause_blink_cursor(cx);
            self.trigger_input();
            self.trigger_change();
            self.validate();
            cx.notify();
            true
        } else {
            false
        }
    }

    pub fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if self.disabled || self.read_only {
            return;
        }

        self.delete_selection(cx);

        let cur_len = self.text.len_chars();
        let add_len = text.chars().count();
        let insert_str = if let Some(max) = self.max_length {
            if cur_len >= max {
                return;
            }
            if cur_len + add_len > max {
                let allowed = max - cur_len;
                text.chars().take(allowed).collect::<String>()
            } else {
                text.to_string()
            }
        } else {
            text.to_string()
        };

        if insert_str.is_empty() {
            return;
        }

        let cursor = self.cursor();
        let old_cursor = cursor;
        self.text.replace_range(cursor..cursor, &insert_str);
        let new_cursor = cursor + insert_str.len();

        self.undo_manager.record_transaction(
            Change::new(
                old_cursor..old_cursor,
                "",
                old_cursor..new_cursor,
                &insert_str,
            ),
            EditIntent::Typing,
        );

        self.selections.active_mut().place_at(new_cursor, None);
        self.pause_blink_cursor(cx);
        self.trigger_input();
        self.trigger_change();
        self.validate();
        cx.notify();
    }

    pub fn backspace(&mut self, word: bool, cx: &mut Context<Self>) {
        if self.disabled || self.read_only {
            return;
        }
        if self.delete_selection(cx) {
            return;
        }
        let cursor = self.cursor();
        if cursor > 0 {
            let prev = if word {
                prev_word_start(&self.text, cursor)
            } else {
                prev_grapheme_boundary(&self.text, cursor)
            };
            let old_text = self.text.slice(prev..cursor).to_string();
            self.undo_manager.record_transaction(
                Change::new(prev..cursor, &old_text, prev..prev, ""),
                EditIntent::Backspace,
            );
            self.text.replace_range(prev..cursor, "");
            self.selections.active_mut().place_at(prev, None);
            self.pause_blink_cursor(cx);
            self.trigger_input();
            self.trigger_change();
            self.validate();
            cx.notify();
        }
    }

    pub fn delete(&mut self, word: bool, cx: &mut Context<Self>) {
        if self.disabled || self.read_only {
            return;
        }
        if self.delete_selection(cx) {
            return;
        }
        let cursor = self.cursor();
        let max = self.text.len();
        if cursor < max {
            let next = if word {
                next_word_end(&self.text, cursor)
            } else {
                next_grapheme_boundary(&self.text, cursor)
            };
            let old_text = self.text.slice(cursor..next).to_string();
            self.undo_manager.record_transaction(
                Change::new(cursor..next, &old_text, cursor..cursor, ""),
                EditIntent::DeleteForward,
            );
            self.text.replace_range(cursor..next, "");
            self.pause_blink_cursor(cx);
            self.trigger_input();
            self.trigger_change();
            self.validate();
            cx.notify();
        }
    }

    pub fn move_left(&mut self, select: bool, word: bool, cx: &mut Context<Self>) {
        let cursor = self.cursor();
        let target = if word {
            prev_word_start(&self.text, cursor)
        } else {
            prev_grapheme_boundary(&self.text, cursor)
        };
        if select {
            let active = self.selections.active_mut();
            active.end = target;
            active.reversed = active.end < active.start;
        } else if self.has_selection() {
            let range = self.selected_range();
            self.selections.active_mut().place_at(range.start, None);
        } else {
            self.selections.active_mut().place_at(target, None);
        }
        self.pause_blink_cursor(cx);
        cx.notify();
    }

    pub fn move_right(&mut self, select: bool, word: bool, cx: &mut Context<Self>) {
        let cursor = self.cursor();
        let target = if word {
            next_word_end(&self.text, cursor)
        } else {
            next_grapheme_boundary(&self.text, cursor)
        };
        if select {
            let active = self.selections.active_mut();
            active.end = target;
            active.reversed = active.end < active.start;
        } else if self.has_selection() {
            let range = self.selected_range();
            self.selections.active_mut().place_at(range.end, None);
        } else {
            self.selections.active_mut().place_at(target, None);
        }
        self.pause_blink_cursor(cx);
        cx.notify();
    }

    pub fn move_up(&mut self, select: bool, cx: &mut Context<Self>) {
        if !M::MULTI_LINE {
            return;
        }
        let cursor = self.cursor();
        let point = self.text.offset_to_point(cursor);
        if point.row > 0 {
            let target_point = super::rope_ext::Point::new(point.row - 1, point.column);
            let target = self.text.point_to_offset(target_point);
            if select {
                let active = self.selections.active_mut();
                active.end = target;
                active.reversed = active.end < active.start;
            } else {
                self.selections.active_mut().place_at(target, None);
            }
            self.pause_blink_cursor(cx);
            cx.notify();
        }
    }

    pub fn move_down(&mut self, select: bool, cx: &mut Context<Self>) {
        if !M::MULTI_LINE {
            return;
        }
        let cursor = self.cursor();
        let point = self.text.offset_to_point(cursor);
        if point.row + 1 < self.text.lines_len() {
            let target_point = super::rope_ext::Point::new(point.row + 1, point.column);
            let target = self.text.point_to_offset(target_point);
            if select {
                let active = self.selections.active_mut();
                active.end = target;
                active.reversed = active.end < active.start;
            } else {
                self.selections.active_mut().place_at(target, None);
            }
            self.pause_blink_cursor(cx);
            cx.notify();
        }
    }

    pub fn move_home(&mut self, select: bool, cx: &mut Context<Self>) {
        let cursor = self.cursor();
        let point = self.text.offset_to_point(cursor);
        let start = self.text.line_start_offset(point.row);
        if select {
            let active = self.selections.active_mut();
            active.end = start;
            active.reversed = active.end < active.start;
        } else {
            self.selections.active_mut().place_at(start, None);
        }
        self.pause_blink_cursor(cx);
        cx.notify();
    }

    pub fn move_end(&mut self, select: bool, cx: &mut Context<Self>) {
        let cursor = self.cursor();
        let point = self.text.offset_to_point(cursor);
        let end = self.text.line_end_offset(point.row);
        if select {
            let active = self.selections.active_mut();
            active.end = end;
            active.reversed = active.end < active.start;
        } else {
            self.selections.active_mut().place_at(end, None);
        }
        self.pause_blink_cursor(cx);
        cx.notify();
    }

    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        let active = self.selections.active_mut();
        active.start = 0;
        active.end = self.text.len();
        active.reversed = false;
        self.pause_blink_cursor(cx);
        cx.notify();
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if let Some(replay) = self.undo_manager.undo() {
            for change in replay.changes {
                self.text.replace_range(change.old_range.start..change.old_range.end, &change.new_text);
            }
            if let Some(selections) = replay.selections {
                self.selections.replace_all(selections);
            }
            self.pause_blink_cursor(cx);
            self.trigger_input();
            self.trigger_change();
            self.validate();
            cx.notify();
        }
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        if let Some(replay) = self.undo_manager.redo() {
            for change in replay.changes {
                self.text.replace_range(change.old_range.start..change.old_range.end, &change.new_text);
            }
            if let Some(selections) = replay.selections {
                self.selections.replace_all(selections);
            }
            self.pause_blink_cursor(cx);
            self.trigger_input();
            self.trigger_change();
            self.validate();
            cx.notify();
        }
    }

    pub fn validate(&mut self) {
        let text_str = self.text.to_string();
        if self.required && text_str.is_empty() {
            self.is_valid = false;
            self.error_msg = Some(String::from("This field is required"));
            return;
        }
        if let Some(max) = self.max_length {
            if self.text.len_chars() > max {
                self.is_valid = false;
                self.error_msg = Some(format!("Cannot exceed {max} characters"));
                return;
            }
        }
        if let Some(pat) = &self.pattern {
            if !text_str.is_empty() {
                let matches = regex::Regex::new(pat).map_or(true, |re| re.is_match(&text_str));
                if !matches {
                    self.is_valid = false;
                    self.error_msg = Some(String::from("Invalid input format"));
                    return;
                }
            }
        }
        self.is_valid = true;
        self.error_msg = None;
    }

    pub fn trigger_change(&self) {
        if let (Some(cb), Some(invoker)) = (&self.on_change, &self.invoker) {
            let _ = invoker.call_key_input(cb, &self.text.to_string());
        }
    }

    pub fn trigger_input(&self) {
        if let (Some(cb), Some(invoker)) = (&self.on_input, &self.invoker) {
            let _ = invoker.call_key_input(cb, &self.text.to_string());
        }
    }

    pub fn trigger_submit(&self) {
        if let (Some(cb), Some(invoker)) = (&self.on_submit, &self.invoker) {
            let _ = invoker.call_key_input(cb, &self.text.to_string());
        }
    }

    pub fn trigger_focus(&self) {
        if let (Some(cb), Some(invoker)) = (&self.on_focus, &self.invoker) {
            let _ = invoker.call_key(cb);
        }
    }

    pub fn trigger_blur(&self) {
        if let (Some(cb), Some(invoker)) = (&self.on_blur, &self.invoker) {
            let _ = invoker.call_key_input(cb, &self.text.to_string());
        }
    }

    pub fn on_mouse_down(&mut self, e: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        window.focus(&self.focus_handle, cx);
        self.blink_cursor.update(cx, |b, cx| {
            b.start(cx);
            b.pause(cx);
        });
        self.trigger_focus();
        if self.text.len() == 0 {
            self.selections.active_mut().place_at(0, None);
            self.pause_blink_cursor(cx);
            cx.notify();
            return;
        }

        let Some(layout) = &self.last_layout else {
            return;
        };
        let Some(bounds) = self.last_bounds else {
            return;
        };

        let pos = e.position;
        let line_height = layout.line_height;
        let rel_y = (pos.y - bounds.top() - layout.scroll_offset.y).max(px(0.0));
        let row = ((rel_y / line_height).floor() as usize).min(layout.lines.len().saturating_sub(1));

        if let Some(line) = layout.lines.get(row) {
            let rel_x = (pos.x - bounds.left() - layout.scroll_offset.x).max(px(0.0));
            let offset_in_line = line.index_for_x(&self.text, rel_x);
            let byte_offset = (line.byte_range.start + offset_in_line).min(self.text.len());
            if e.click_count == 2 {
                if let Some(range) = TextSelector::word_range(&self.text, byte_offset) {
                    let active = self.selections.active_mut();
                    active.start = range.start;
                    active.end = range.end;
                    active.reversed = false;
                }
            } else if e.click_count >= 3 {
                let range = TextSelector::line_range(&self.text, byte_offset);
                let active = self.selections.active_mut();
                active.start = range.start;
                active.end = range.end;
                active.reversed = false;
            } else {
                self.selecting = true;
                self.selections.active_mut().place_at(byte_offset, None);
            }
        }

        self.pause_blink_cursor(cx);
        cx.notify();
    }

    pub fn on_mouse_move(&mut self, e: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.selecting || self.disabled {
            return;
        }
        if self.text.len() == 0 {
            self.selections.active_mut().place_at(0, None);
            cx.notify();
            return;
        }

        if e.pressed_button != Some(MouseButton::Left) {
            self.selecting = false;
            let active = self.selections.active_mut();
            if active.start == active.end {
                active.reversed = false;
            }
            cx.notify();
            return;
        }
        let Some(layout) = &self.last_layout else {
            return;
        };
        let Some(bounds) = self.last_bounds else {
            return;
        };

        let pos = e.position;
        let line_height = layout.line_height;
        let rel_y = (pos.y - bounds.top() - layout.scroll_offset.y).max(px(0.0));
        let row = ((rel_y / line_height).floor() as usize).min(layout.lines.len().saturating_sub(1));

        if let Some(line) = layout.lines.get(row) {
            let rel_x = (pos.x - bounds.left() - layout.scroll_offset.x).max(px(0.0));
            let offset_in_line = line.index_for_x(&self.text, rel_x);
            let byte_offset = line.byte_range.start + offset_in_line;
            let active = self.selections.active_mut();
            active.end = byte_offset;
            active.reversed = active.end < active.start;
            cx.notify();
        }
    }

    pub fn on_mouse_up(&mut self, _e: &MouseUpEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.selecting = false;
        let active = self.selections.active_mut();
        if active.start == active.end {
            active.reversed = false;
        }
        cx.notify();
    }

    pub fn on_scroll_wheel(&mut self, _e: &ScrollWheelEvent, _window: &mut Window, cx: &mut Context<Self>) {
        cx.notify();
    }

    pub fn handle_key_down(&mut self, e: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }

        let m = &e.keystroke.modifiers;
        let key = e.keystroke.key.to_lowercase();
        let is_ctrl = m.control || m.platform;
        let is_shift = m.shift;

        match key.as_str() {
            "a" if is_ctrl => self.select_all(cx),
            "c" if is_ctrl => {
                if self.has_selection() {
                    let range = self.selected_range();
                    let sel = self.text.slice(range).to_string();
                    cx.write_to_clipboard(ClipboardItem::new_string(sel));
                }
            }
            "x" if is_ctrl && !self.read_only => {
                if self.has_selection() {
                    let range = self.selected_range();
                    let sel = self.text.slice(range).to_string();
                    cx.write_to_clipboard(ClipboardItem::new_string(sel));
                    self.delete_selection(cx);
                }
            }
            "v" if is_ctrl && !self.read_only => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.insert_text(&text, cx);
                }
            }
            "z" if is_ctrl && !is_shift => self.undo(cx),
            "y" if is_ctrl => self.redo(cx),
            "z" if is_ctrl && is_shift => self.redo(cx),
            "backspace" => self.backspace(is_ctrl, cx),
            "delete" => self.delete(is_ctrl, cx),
            "left" => self.move_left(is_shift, is_ctrl, cx),
            "right" => self.move_right(is_shift, is_ctrl, cx),
            "up" => self.move_up(is_shift, cx),
            "down" => self.move_down(is_shift, cx),
            "home" => self.move_home(is_shift, cx),
            "end" => self.move_end(is_shift, cx),
            "enter" => {
                if M::MULTI_LINE && !is_ctrl {
                    if let Some(max_r) = self.max_rows {
                        if self.text.lines_len() >= max_r && !self.has_selection() {
                            return;
                        }
                    }
                    self.insert_text("\n", cx);
                } else {
                    self.trigger_submit();
                }
            }
            "tab" => {
                if M::MULTI_LINE && !self.read_only {
                    self.insert_text("  ", cx);
                }
            }
            "escape" => {
                window.focus(&cx.focus_handle(), cx);
            }
            _ => {}
        }
    }

    pub fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.text.offset_to_utf16(range.start)..self.text.offset_to_utf16(range.end)
    }

    pub fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.text.offset_from_utf16(range_utf16.start)..self.text.offset_from_utf16(range_utf16.end)
    }
}

impl<M: InputModeKind> EntityInputHandler for InputBaseState<M> {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let mut range = self.range_from_utf16(&range_utf16);
        let len = self.text.len();
        range.start = self.text.clip_offset(range.start.min(len), true);
        range.end = self.text.clip_offset(range.end.min(len), false).max(range.start);
        adjusted_range.replace(self.range_to_utf16(&range));
        Some(self.text.slice(range).to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let range = self.selected_range();
        Some(UTF16Selection {
            range: self.range_to_utf16(&range),
            reversed: self.selections.active().reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.ime_marked_range.as_ref().map(|sel| {
            let r = sel.start..sel.end;
            self.range_to_utf16(&r)
        })
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.ime_marked_range = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled || self.read_only {
            return;
        }

        let mut range = range_utf16
            .map(|r| self.range_from_utf16(&r))
            .or_else(|| self.ime_marked_range.as_ref().map(|sel| sel.start..sel.end))
            .unwrap_or_else(|| self.selected_range());

        let len = self.text.len();
        range.start = self.text.clip_offset(range.start.min(len), true);
        range.end = self.text.clip_offset(range.end.min(len), false).max(range.start);

        let old_text = self.text.slice(range.clone()).to_string();
        self.undo_manager.record_transaction(
            Change::new(range.clone(), &old_text, range.start..range.start + new_text.len(), new_text),
            EditIntent::Typing,
        );

        self.text.replace_range(range.clone(), new_text);

        let new_cursor = range.start + new_text.len();
        self.selections.active_mut().place_at(new_cursor, None);
        self.ime_marked_range = None;
        self.pause_blink_cursor(cx);
        self.trigger_input();
        self.trigger_change();
        self.validate();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled || self.read_only {
            return;
        }

        let range = range_utf16
            .map(|r| self.range_from_utf16(&r))
            .or_else(|| self.ime_marked_range.as_ref().map(|sel| sel.start..sel.end))
            .unwrap_or_else(|| self.selected_range());

        self.text.replace_range(range.clone(), new_text);

        let marked_start = range.start;
        let marked_end = marked_start + new_text.len();
        self.ime_marked_range = Some(CursorSelection::new(CursorId::default(), marked_start, marked_end));

        if let Some(new_sel) = new_selected_range_utf16 {
            let start = marked_start + new_sel.start;
            let end = marked_start + new_sel.end;
            let active = self.selections.active_mut();
            active.start = start;
            active.end = end;
            active.reversed = false;
        } else {
            self.selections.active_mut().place_at(marked_end, None);
        }

        self.pause_blink_cursor(cx);
        self.trigger_input();
        self.trigger_change();
        self.validate();
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        self.last_bounds.or(Some(element_bounds))
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}
