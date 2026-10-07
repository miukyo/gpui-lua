use super::blink_cursor::CURSOR_WIDTH;
use super::kind::InputModeKind;
use super::layout::{LastLayout, LineLayout};
use super::mask_pattern::MASK_CHAR;
use super::rope_ext::RopeExt;
use super::state::InputBaseState;
use gpui::{
    fill, point, px, relative, size, App, Bounds, ColorExt, ContentMask, Element, ElementId,
    ElementInputHandler, Entity, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    LineFragment, PaintQuad, Pixels, Point, SharedString, Style, TextRun, Window,
};
use std::ops::Range;

pub struct TextElement<M: InputModeKind> {
    pub state: Entity<InputBaseState<M>>,
}

impl<M: InputModeKind> TextElement<M> {
    pub fn new(state: Entity<InputBaseState<M>>) -> Self {
        Self { state }
    }
}

pub struct TextElementPrepaint {
    _bounds: Bounds<Pixels>,
    last_layout: LastLayout,
    selection_quads: Vec<PaintQuad>,
    cursor_quad: Option<PaintQuad>,
    scroll_offset: Point<Pixels>,
}

impl<M: InputModeKind> IntoElement for TextElement<M> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<M: InputModeKind> Element for TextElement<M> {
    type RequestLayoutState = ();
    type PrepaintState = TextElementPrepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let state = self.state.read(cx);
        let line_height = px(state.font_size * 1.35).max(px(16.0));
        let total_lines = if M::MULTI_LINE {
            let mut total = state.text.lines_len().max(state.rows);
            if let Some(max_r) = state.max_rows {
                total = total.min(max_r);
            }
            total
        } else {
            1
        };

        let mut style = Style::default();
        if let Some(w) = state.width {
            style.size.width = w.into();
        } else {
            style.size.width = relative(1.0).into();
        }

        if let Some(h) = state.height {
            style.size.height = h.into();
        } else {
            style.size.height = (line_height * total_lines as f32).into();
        }

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let state = self.state.read(cx);
        let focus_handle = state.focus_handle.clone();
        let is_focused = focus_handle.is_focused(window);
        let show_cursor = is_focused && state.blink_cursor.read(cx).visible() && !state.disabled;

        let mut style = window.text_style();
        if let Some(ref ff) = state.font_family {
            style.font_family = gpui::SharedString::from(ff.clone());
        }
        let font_size = if state.font_size > 0.0 { px(state.font_size) } else { style.font_size.to_pixels(window.rem_size()) };
        let line_height = px(state.font_size * 1.35).max(px(16.0));
        let text_color = state.text_color.unwrap_or_else(|| gpui::rgb_to_hsla(gpui::rgb(0xcdd6f4)));
        let placeholder_color = state
            .placeholder_color
            .unwrap_or_else(|| gpui::rgb_to_hsla(gpui::rgb(0x6c7086)));
        let cursor_color = state
            .cursor_color
            .unwrap_or_else(|| gpui::rgb_to_hsla(gpui::rgb(0x89b4fa)));
        let selection_bg = state
            .selection_bg
            .unwrap_or_else(|| gpui::rgb_to_hsla(gpui::rgb(0x313244)));

        let is_empty = state.text.len() == 0;
        let selected_range = state.selected_range();
        let cursor_offset = state.cursor();

        let mut lines = Vec::new();
        let mut selection_quads = Vec::new();
        let mut cursor_quad = None;
        let mut cursor_visual_pos: Option<Point<Pixels>> = None;

        if is_empty && !is_focused {
            let placeholder_str = SharedString::from(state.placeholder.clone());
            let run = TextRun {
                len: placeholder_str.len(),
                font: style.font(),
                color: placeholder_color,
                background_color: None,
                underline: None,
                strikethrough: None,
                letter_spacing: None,
            };
            let shaped = window.text_system().shape_line(placeholder_str, font_size, &[run], None);
            lines.push(LineLayout {
                shaped_line: shaped,
                row: 0,
                byte_range: 0..0,
                width: bounds.size.width,
                height: line_height,
                is_masked: false,
            });
        } else {
            let logical_lines_count = state.text.lines_len().max(1);
            let mut visual_row = 0;

            for log_row in 0..logical_lines_count {
                let log_start = state.text.line_start_offset(log_row);
                let _log_end = state.text.line_end_offset(log_row);
                let line_str = state.text.slice_line(log_row).to_string();

                // Compute segments: either wrapped for textarea, or single segment for input
                let segments: Vec<Range<usize>> = if M::MULTI_LINE && bounds.size.width > px(40.0) {
                    let mut wrapper = window.text_system().line_wrapper(style.font(), font_size);
                    let boundaries: Vec<_> = wrapper
                        .wrap_line(&[LineFragment::text(&line_str)], bounds.size.width)
                        .collect();

                    if boundaries.is_empty() {
                        vec![0..line_str.len()]
                    } else {
                        let mut segs = Vec::new();
                        let mut last_ix = 0;
                        for b in boundaries {
                            segs.push(last_ix..b.ix);
                            last_ix = b.ix;
                        }
                        if last_ix < line_str.len() {
                            segs.push(last_ix..line_str.len());
                        }
                        segs
                    }
                } else {
                    vec![0..line_str.len()]
                };

                for seg in segments {
                    let seg_text = &line_str[seg.clone()];
                    let seg_start = log_start + seg.start;
                    let seg_end = log_start + seg.end;

                    let (disp_str, is_masked) = if state.masked {
                        let count = seg_text.chars().count();
                        (std::iter::repeat(MASK_CHAR).take(count).collect::<String>(), true)
                    } else {
                        (seg_text.to_string(), false)
                    };

                    let shared_disp = SharedString::from(disp_str.clone());
                    let run = TextRun {
                        len: shared_disp.len(),
                        font: style.font(),
                        color: text_color,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                        letter_spacing: None,
                    };

                    let shaped = window.text_system().shape_line(shared_disp, font_size, &[run], None);
                    let line_w = shaped.width;

                    // Selection highlight quad for this segment
                    if !selected_range.is_empty() {
                        let s_start = selected_range.start.max(seg_start).min(seg_end);
                        let s_end = selected_range.end.max(seg_start).min(seg_end);
                        if s_start < s_end {
                            let off_start = s_start - seg_start;
                            let off_end = s_end - seg_start;
                            let x1_disp = if is_masked {
                                let c_count = seg_text[..off_start.min(seg_text.len())].chars().count();
                                c_count * MASK_CHAR.len_utf8()
                            } else {
                                off_start
                            };
                            let x2_disp = if is_masked {
                                let c_count = seg_text[..off_end.min(seg_text.len())].chars().count();
                                c_count * MASK_CHAR.len_utf8()
                            } else {
                                off_end
                            };

                            let x1 = shaped.x_for_index(x1_disp);
                            let x2 = shaped.x_for_index(x2_disp);
                            let quad_b = Bounds::new(
                                point(x1, line_height * visual_row as f32),
                                size(x2 - x1, line_height),
                            );
                            selection_quads.push(fill(quad_b, selection_bg));
                        }
                    }

                    // Cursor position check
                    let cursor_in_this_seg = if cursor_offset >= seg_start && cursor_offset <= seg_end {
                        if cursor_offset == seg_end && seg.end < line_str.len() {
                            false // Cursor lands on start of next wrapped row
                        } else {
                            true
                        }
                    } else {
                        false
                    };

                    if cursor_in_this_seg {
                        let off_in_seg = cursor_offset.saturating_sub(seg_start).min(seg_text.len());
                        let disp_off = if is_masked {
                            let c_count = seg_text[..off_in_seg].chars().count();
                            c_count * MASK_CHAR.len_utf8()
                        } else {
                            off_in_seg
                        };
                        let cursor_x = shaped.x_for_index(disp_off);
                        let cursor_y = line_height * visual_row as f32;
                        cursor_visual_pos = Some(point(cursor_x, cursor_y));

                        if show_cursor {
                            let cursor_b = Bounds::new(
                                point(cursor_x, cursor_y),
                                size(CURSOR_WIDTH, line_height),
                            );
                            cursor_quad = Some(fill(cursor_b, cursor_color));
                        }
                    }

                    lines.push(LineLayout {
                        shaped_line: shaped,
                        row: visual_row,
                        byte_range: seg_start..seg_end,
                        width: line_w,
                        height: line_height,
                        is_masked,
                    });

                    visual_row += 1;
                }
            }

            // Faded placeholder when focused and completely empty
            if is_empty && is_focused && !state.placeholder.is_empty() {
                let placeholder_str = SharedString::from(state.placeholder.clone());
                let run = TextRun {
                    len: placeholder_str.len(),
                    font: style.font(),
                    color: placeholder_color.opacity(0.4),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                    letter_spacing: None,
                };
                let shaped = window.text_system().shape_line(placeholder_str, font_size, &[run], None);
                lines.push(LineLayout {
                    shaped_line: shaped,
                    row: 0,
                    byte_range: 0..0,
                    width: bounds.size.width,
                    height: line_height,
                    is_masked: false,
                });
            }
        }

        // Compute scroll offset to keep cursor visible in viewport
        let mut scroll_offset = point(px(0.0), px(0.0));
        let avail_w = bounds.size.width;
        let avail_h = bounds.size.height;

        if let Some(cpos) = cursor_visual_pos {
            if !M::MULTI_LINE {
                // Horizontal scrolling for single-line input
                if cpos.x > avail_w - px(16.0) {
                    scroll_offset.x = avail_w - px(16.0) - cpos.x;
                } else {
                    scroll_offset.x = px(0.0);
                }
            } else {
                // Vertical scrolling for textarea
                let total_content_h = line_height * lines.len() as f32;
                if total_content_h > avail_h {
                    if cpos.y + line_height > avail_h {
                        scroll_offset.y = avail_h - cpos.y - line_height;
                    }
                    let min_scroll = avail_h - total_content_h;
                    scroll_offset.y = scroll_offset.y.clamp(min_scroll, px(0.0));
                } else {
                    scroll_offset.y = px(0.0);
                }
            }
        }

        let last_layout = LastLayout {
            lines,
            visible_range: 0..state.text.len(),
            content_size: bounds.size,
            line_height,
            text_align: gpui::TextAlign::Left,
            space_width: px(8.0),
            scroll_offset,
        };

        TextElementPrepaint {
            _bounds: bounds,
            last_layout,
            selection_quads,
            cursor_quad,
            scroll_offset,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.state.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.state.clone()),
            cx,
        );

        // Update state geometry with exact bounds and layout
        self.state.update(cx, |st, _| {
            st.last_bounds = Some(bounds);
            st.last_layout = Some(prepaint.last_layout.clone());
        });

        let scroll_offset = prepaint.scroll_offset;
        let origin = bounds.origin + scroll_offset;
        let line_height = prepaint.last_layout.line_height;

        // Clip text and highlights cleanly inside the bounds
        window.with_content_mask(Some(ContentMask { bounds, fade_out: Default::default() }), |window| {
            // 1. Paint selection highlight quads
            for quad in &prepaint.selection_quads {
                let mut q = quad.clone();
                q.bounds.origin = origin + quad.bounds.origin;
                window.paint_quad(q);
            }

            // 2. Paint shaped text lines
            for line in &prepaint.last_layout.lines {
                let p = point(origin.x, origin.y + line_height * line.row as f32);
                let _ = line.shaped_line.paint(
                    p,
                    line_height,
                    gpui::TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }

            // 3. Paint blinking cursor
            if let Some(mut cursor) = prepaint.cursor_quad.take() {
                cursor.bounds.origin = origin + cursor.bounds.origin;
                window.paint_quad(cursor);
            }
        });
    }
}
