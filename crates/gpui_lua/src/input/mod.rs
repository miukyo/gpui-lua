pub mod blink_cursor;
pub mod change;
pub mod cursor;
pub mod element;
pub mod kind;
pub mod layout;
pub mod mask_pattern;
pub mod movement;
pub mod rope_ext;
pub mod selection;
pub mod state;
pub mod text_boundary;
pub mod undo_manager;

pub use blink_cursor::BlinkCursor;
pub use change::Change;
pub use cursor::{CursorId, CursorSelection, Selection, Selections};
pub use element::TextElement;
pub use kind::{InputMode, InputModeKind, TextareaMode};
pub use mask_pattern::{MaskPattern, MaskToken, MASK_CHAR};
pub use rope_ext::{Point, RopeExt};
pub use state::InputBaseState;
pub use undo_manager::{EditIntent, UndoManager};

pub type InputState = InputBaseState<InputMode>;
pub type TextareaState = InputBaseState<TextareaMode>;

use crate::dsl::convert::LuaInvoker;
use crate::dsl::node::InputNode;
use gpui::{
    div, px, rgb, App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window,
};
use std::sync::Arc;

/// RenderOnce element for single-line Input
#[derive(IntoElement)]
pub struct InputElement {
    pub node: InputNode,
    pub invoker: Option<Arc<dyn LuaInvoker>>,
}

impl InputElement {
    pub fn new(node: InputNode, invoker: Option<Arc<dyn LuaInvoker>>) -> Self {
        Self { node, invoker }
    }
}

impl RenderOnce for InputElement {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let node = self.node;
        let invoker = self.invoker;
        let id_str = node.id.clone();

        let state = window.use_keyed_state(
            ElementId::Name(SharedString::from(format!("input_{}", id_str))),
            cx,
            |window, cx| {
                let mut st = InputBaseState::<InputMode>::new(window, cx);
                if let Some(val) = &node.value {
                    st.last_external_value = Some(val.clone());
                    st.set_text(val, cx);
                }
                if node.autofocus {
                    window.focus(&st.focus_handle, &mut **cx);
                }
                st
            },
        );

        // Synchronize dynamic properties onto state
        state.update(cx, |st, cx| {
            if let Some(val) = &node.value {
                if st.last_external_value.as_deref() != Some(val) {
                    st.last_external_value = Some(val.clone());
                    st.set_text(val, cx);
                }
            }
            st.placeholder = node.placeholder.clone();
            st.masked = node.mask.is_some();
            st.max_length = node.max_length;
            st.pattern = node.pattern.clone();
            st.required = node.required;
            st.disabled = node.disabled;
            st.read_only = node.read_only;
            st.on_change = node.on_change.clone();
            st.on_input = node.on_input.clone();
            st.on_submit = node.on_submit.clone();
            st.on_focus = node.on_focus.clone();
            st.on_blur = node.on_blur.clone();
            st.on_validate = node.on_validate.clone();
            st.invoker = invoker.clone();

            if let Some(c) = &node.style.text_color {
                st.text_color = Some(c.to_hsla());
            }
            if let Some(ff) = &node.style.font_family {
                st.font_family = Some(ff.clone());
            }
            if let Some(c) = &node.placeholder_color {
                st.placeholder_color = Some(c.to_hsla());
            }
            if let Some(c) = &node.cursor_color {
                st.cursor_color = Some(c.to_hsla());
            }
            if let Some(c) = &node.selection_bg {
                st.selection_bg = Some(c.to_hsla());
            }
            if let Some(sz) = node.font_size {
                st.font_size = sz;
            }
            st.validate();
        });

        let (focus_handle, is_valid, error_msg) = {
            let st = state.read(cx);
            (st.focus_handle.clone(), st.is_valid, st.error_msg.clone())
        };

        let is_focused = focus_handle.is_focused(window);

        // Resolve colors
        let bg = if is_focused {
            node.focus_bg
                .as_ref()
                .map(|c| c.to_hsla())
                .or_else(|| node.style.background.as_ref().map(|c| c.to_hsla()))
                .unwrap_or(gpui::rgb_to_hsla(rgb(0x1e1e2e)))
        } else {
            node.style
                .background
                .as_ref()
                .map(|c| c.to_hsla())
                .unwrap_or(gpui::rgb_to_hsla(rgb(0x181825)))
        };

        let border_color = if !is_valid {
            node.error_border_color
                .as_ref()
                .map(|c| c.to_hsla())
                .unwrap_or(gpui::rgb_to_hsla(rgb(0xef4444)))
        } else if is_focused {
            node.focus_border_color
                .as_ref()
                .map(|c| c.to_hsla())
                .unwrap_or(gpui::rgb_to_hsla(rgb(0x89b4fa)))
        } else {
            node.style
                .border_color
                .as_ref()
                .map(|c| c.to_hsla())
                .unwrap_or(gpui::rgb_to_hsla(rgb(0x45475a)))
        };

        let border_w = node.style.border_width.unwrap_or(1.0);
        let rounded = node.style.corner_radius.unwrap_or(6.0);
        let px_pad = node
            .style
            .padding
            .left
            .and_then(|l| match l {
                crate::dsl::node::Length::Px(p) => Some(p),
                _ => None,
            })
            .unwrap_or(12.0);
        let py_pad = node
            .style
            .padding
            .top
            .and_then(|l| match l {
                crate::dsl::node::Length::Px(p) => Some(p),
                _ => None,
            })
            .unwrap_or(8.0);

        let mut field_div = div()
            .id(SharedString::from(format!("input_frame_{}", id_str)))
            .track_focus(&focus_handle)
            .focusable()
            .tab_index(0)
            .bg(bg)
            .border(px(border_w))
            .border_color(border_color)
            .rounded(px(rounded))
            .px(px(px_pad))
            .py(px(py_pad))
            .text_color(node.style.text_color.as_ref().map(|c| c.to_hsla()).unwrap_or_else(|| gpui::rgb_to_hsla(gpui::rgb(0xcdd6f4))))
            .flex()
            .items_center()
            .on_mouse_down(MouseButton::Left, {
                let state = state.clone();
                move |e, window, cx| {
                    state.update(cx, |st, cx| st.on_mouse_down(e, window, cx));
                }
            })
            .on_mouse_move({
                let state = state.clone();
                move |e, window, cx| {
                    state.update(cx, |st, cx| st.on_mouse_move(e, window, cx));
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let state = state.clone();
                move |e, window, cx| {
                    state.update(cx, |st, cx| st.on_mouse_up(e, window, cx));
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let state = state.clone();
                move |e, window, cx| {
                    state.update(cx, |st, cx| st.on_mouse_up(e, window, cx));
                }
            })
            .on_key_down({
                let state = state.clone();
                move |e, window, cx| {
                    state.update(cx, |st, cx| st.handle_key_down(e, window, cx));
                }
            })
            .child(TextElement::new(state.clone()));

        if let Some(op) = node.style.opacity {
            field_div = field_div.opacity(op);
        }
        if let Some(sh) = node.style.shadow {
            match sh {
                crate::dsl::node::ShadowLevel::Sm => field_div = field_div.shadow_sm(),
                crate::dsl::node::ShadowLevel::Md => field_div = field_div.shadow_md(),
                crate::dsl::node::ShadowLevel::Lg => field_div = field_div.shadow_lg(),
            }
        }
        if let Some(ff) = &node.style.font_family {
            field_div = field_div.font_family(SharedString::from(ff.clone()));
        }
        if let Some(w) = node.style.width {
            match w {
                crate::dsl::node::Length::Px(p) => field_div = field_div.w(px(p)),
                crate::dsl::node::Length::Full => field_div = field_div.w_full(),
                _ => field_div = field_div.w_full(),
            }
        } else {
            field_div = field_div.w_full();
        }

        if let Some(h) = node.style.height {
            match h {
                crate::dsl::node::Length::Px(p) => field_div = field_div.h(px(p)),
                crate::dsl::node::Length::Full => field_div = field_div.h_full(),
                _ => {}
            }
        }

        if !is_valid && error_msg.is_some() {
            let err = error_msg.unwrap();
            let err_c = node
                .error_border_color
                .as_ref()
                .map(|c| c.to_hsla())
                .unwrap_or(gpui::rgb_to_hsla(rgb(0xef4444)));
            div()
                .flex_col()
                .gap(px(4.0))
                .w_full()
                .child(field_div)
                .child(
                    div()
                        .px(px(2.0))
                        .text_size(px(12.0))
                        .text_color(err_c)
                        .child(SharedString::from(err)),
                )
                .into_any_element()
        } else {
            field_div.into_any_element()
        }
    }
}

/// RenderOnce element for multi-line Textarea
#[derive(IntoElement)]
pub struct TextareaElement {
    pub node: InputNode,
    pub invoker: Option<Arc<dyn LuaInvoker>>,
}

impl TextareaElement {
    pub fn new(node: InputNode, invoker: Option<Arc<dyn LuaInvoker>>) -> Self {
        Self { node, invoker }
    }
}

impl RenderOnce for TextareaElement {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let node = self.node;
        let invoker = self.invoker;
        let id_str = node.id.clone();
        let rows = node.rows.max(1);

        let state = window.use_keyed_state(
            ElementId::Name(SharedString::from(format!("textarea_{}", id_str))),
            cx,
            |window, cx| {
                let mut st = InputBaseState::<TextareaMode>::new(window, cx);
                st.rows = rows;
                if let Some(val) = &node.value {
                    st.last_external_value = Some(val.clone());
                    st.set_text(val, cx);
                }
                if node.autofocus {
                    window.focus(&st.focus_handle, &mut **cx);
                }
                st
            },
        );

        state.update(cx, |st, cx| {
            if let Some(val) = &node.value {
                if st.last_external_value.as_deref() != Some(val) {
                    st.last_external_value = Some(val.clone());
                    st.set_text(val, cx);
                }
            }
            st.rows = rows;
            st.max_rows = node.max_rows;
            st.placeholder = node.placeholder.clone();
            st.max_length = node.max_length;
            st.pattern = node.pattern.clone();
            st.required = node.required;
            st.disabled = node.disabled;
            st.read_only = node.read_only;
            st.on_change = node.on_change.clone();
            st.on_input = node.on_input.clone();
            st.on_submit = node.on_submit.clone();
            st.on_focus = node.on_focus.clone();
            st.on_blur = node.on_blur.clone();
            st.on_validate = node.on_validate.clone();
            st.invoker = invoker.clone();

            if let Some(c) = &node.style.text_color {
                st.text_color = Some(c.to_hsla());
            }
            if let Some(ff) = &node.style.font_family {
                st.font_family = Some(ff.clone());
            }
            if let Some(c) = &node.placeholder_color {
                st.placeholder_color = Some(c.to_hsla());
            }
            if let Some(c) = &node.cursor_color {
                st.cursor_color = Some(c.to_hsla());
            }
            if let Some(c) = &node.selection_bg {
                st.selection_bg = Some(c.to_hsla());
            }
            if let Some(sz) = node.font_size {
                st.font_size = sz;
            }
            st.validate();
        });

        let (focus_handle, is_valid, error_msg, font_size) = {
            let st = state.read(cx);
            (st.focus_handle.clone(), st.is_valid, st.error_msg.clone(), st.font_size)
        };

        let is_focused = focus_handle.is_focused(window);

        let bg = if is_focused {
            node.focus_bg
                .as_ref()
                .map(|c| c.to_hsla())
                .or_else(|| node.style.background.as_ref().map(|c| c.to_hsla()))
                .unwrap_or(gpui::rgb_to_hsla(rgb(0x1e1e2e)))
        } else {
            node.style
                .background
                .as_ref()
                .map(|c| c.to_hsla())
                .unwrap_or(gpui::rgb_to_hsla(rgb(0x181825)))
        };

        let border_color = if !is_valid {
            node.error_border_color
                .as_ref()
                .map(|c| c.to_hsla())
                .unwrap_or(gpui::rgb_to_hsla(rgb(0xef4444)))
        } else if is_focused {
            node.focus_border_color
                .as_ref()
                .map(|c| c.to_hsla())
                .unwrap_or(gpui::rgb_to_hsla(rgb(0x89b4fa)))
        } else {
            node.style
                .border_color
                .as_ref()
                .map(|c| c.to_hsla())
                .unwrap_or(gpui::rgb_to_hsla(rgb(0x45475a)))
        };

        let border_w = node.style.border_width.unwrap_or(1.0);
        let rounded = node.style.corner_radius.unwrap_or(6.0);
        let px_pad = node
            .style
            .padding
            .left
            .and_then(|l| match l {
                crate::dsl::node::Length::Px(p) => Some(p),
                _ => None,
            })
            .unwrap_or(12.0);
        let py_pad = node
            .style
            .padding
            .top
            .and_then(|l| match l {
                crate::dsl::node::Length::Px(p) => Some(p),
                _ => None,
            })
            .unwrap_or(8.0);

        let min_h = (rows as f32) * (font_size + 6.0) + py_pad * 2.0;

        let mut field_div = div()
            .id(SharedString::from(format!("textarea_frame_{}", id_str)))
            .track_focus(&focus_handle)
            .focusable()
            .tab_index(0)
            .bg(bg)
            .border(px(border_w))
            .border_color(border_color)
            .rounded(px(rounded))
            .px(px(px_pad))
            .py(px(py_pad))
            .text_color(node.style.text_color.as_ref().map(|c| c.to_hsla()).unwrap_or_else(|| gpui::rgb_to_hsla(gpui::rgb(0xcdd6f4))))
            .min_h(px(min_h))
            .flex_col();

        if let Some(max_r) = node.max_rows {
            let max_h = (max_r as f32) * (font_size + 6.0) + py_pad * 2.0;
            field_div = field_div.max_h(px(max_h));
        }

        field_div = field_div
            .on_mouse_down(MouseButton::Left, {
                let state = state.clone();
                move |e, window, cx| {
                    state.update(cx, |st, cx| st.on_mouse_down(e, window, cx));
                }
            })
            .on_mouse_move({
                let state = state.clone();
                move |e, window, cx| {
                    state.update(cx, |st, cx| st.on_mouse_move(e, window, cx));
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let state = state.clone();
                move |e, window, cx| {
                    state.update(cx, |st, cx| st.on_mouse_up(e, window, cx));
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let state = state.clone();
                move |e, window, cx| {
                    state.update(cx, |st, cx| st.on_mouse_up(e, window, cx));
                }
            })
            .on_scroll_wheel({
                let state = state.clone();
                move |e, window, cx| {
                    state.update(cx, |st, cx| st.on_scroll_wheel(e, window, cx));
                }
            })
            .on_key_down({
                let state = state.clone();
                move |e, window, cx| {
                    state.update(cx, |st, cx| st.handle_key_down(e, window, cx));
                }
            })
            .child(TextElement::new(state.clone()));

        if let Some(op) = node.style.opacity {
            field_div = field_div.opacity(op);
        }
        if let Some(sh) = node.style.shadow {
            match sh {
                crate::dsl::node::ShadowLevel::Sm => field_div = field_div.shadow_sm(),
                crate::dsl::node::ShadowLevel::Md => field_div = field_div.shadow_md(),
                crate::dsl::node::ShadowLevel::Lg => field_div = field_div.shadow_lg(),
            }
        }
        if let Some(ff) = &node.style.font_family {
            field_div = field_div.font_family(SharedString::from(ff.clone()));
        }
        if let Some(w) = node.style.width {
            match w {
                crate::dsl::node::Length::Px(p) => field_div = field_div.w(px(p)),
                crate::dsl::node::Length::Full => field_div = field_div.w_full(),
                _ => field_div = field_div.w_full(),
            }
        } else {
            field_div = field_div.w_full();
        }

        if let Some(h) = node.style.height {
            match h {
                crate::dsl::node::Length::Px(p) => field_div = field_div.h(px(p)),
                crate::dsl::node::Length::Full => field_div = field_div.h_full(),
                _ => {}
            }
        }

        if !is_valid && error_msg.is_some() {
            let err = error_msg.unwrap();
            let err_c = node
                .error_border_color
                .as_ref()
                .map(|c| c.to_hsla())
                .unwrap_or(gpui::rgb_to_hsla(rgb(0xef4444)));
            div()
                .flex_col()
                .gap(px(4.0))
                .w_full()
                .child(field_div)
                .child(
                    div()
                        .px(px(2.0))
                        .text_size(px(12.0))
                        .text_color(err_c)
                        .child(SharedString::from(err)),
                )
                .into_any_element()
        } else {
            field_div.into_any_element()
        }
    }
}
