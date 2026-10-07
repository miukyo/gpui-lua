use crate::devtools::state::{
    DevToolsState, DevToolsTab, ElementsSubTab, LogLevel, NetworkType, StorageSubTab,
};
use crate::runtime::LuaRuntime;
use gpui::{
    div, px, rgb, rgba, AnyElement, Context, ElementId, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
};
use std::sync::Arc;

pub struct DevToolsView {
    pub state: Arc<DevToolsState>,
    pub runtime: Arc<LuaRuntime>,
    focus_handle: FocusHandle,
}

impl DevToolsView {
    pub fn new(
        state: Arc<DevToolsState>,
        runtime: Arc<LuaRuntime>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            state,
            runtime,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Render for DevToolsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab = *self.state.active_tab.read();
        let inspect_active = self.state.inspect_cursor_active.load(std::sync::atomic::Ordering::Relaxed);

        div()
            .id("gpui_devtools_window")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(rgb(0x181825))
            .text_color(rgb(0xcdd6f4))
            .flex()
            .flex_col()
            .overflow_hidden()
            // 1. Top Horizontal Main Navbar
            .child(self.render_navbar(active_tab, inspect_active, cx))
            // 2. Active Tab Content (Full Remaining Viewport)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(match active_tab {
                        DevToolsTab::Elements => self.render_elements_tab(cx).into_any_element(),
                        DevToolsTab::Network => self.render_network_tab(cx).into_any_element(),
                        DevToolsTab::Storage => self.render_storage_tab(cx).into_any_element(),
                        DevToolsTab::Console => self.render_console_tab(cx).into_any_element(),
                        DevToolsTab::Performance => self.render_performance_tab(cx).into_any_element(),
                    }),
            )
    }
}

impl DevToolsView {
    // ── Top Horizontal Navbar ───────────────────────────────────────────────────
    fn render_navbar(&self, active_tab: DevToolsTab, inspect_active: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let state_inspect = self.state.clone();
        let bridge_inspect = self.runtime.bridge().clone();
        let state_tabs = self.state.clone();

        let tabs = [
            (DevToolsTab::Elements, "Elements"),
            (DevToolsTab::Console, "Console"),
            (DevToolsTab::Network, "Network"),
            (DevToolsTab::Storage, "Storage"),
            (DevToolsTab::Performance, "Performance"),
        ];

        let fps = *self.state.fps.read();
        let node_count = self.state.node_bounds.read().len();

        div()
            .h(px(36.0))
            .w_full()
            .bg(rgb(0x11111b))
            .border_b_1()
            .border_color(rgb(0x313244))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(6.0))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(2.0))
                    // Inspect Cursor Toggle Button
                    .child(
                        div()
                            .id("devtools_inspect_btn")
                            .px(px(8.0))
                            .py(px(4.0))
                            .mr(px(4.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .bg(if inspect_active { rgb(0x3b82f6) } else { rgb(0x313244) })
                            .text_color(if inspect_active { rgb(0x11111b) } else { rgb(0xcdd6f4) })
                            .text_size(px(12.0))
                            .on_click(cx.listener(move |_this, _event, _window, cx| {
                                let new_val = !state_inspect.inspect_cursor_active.load(std::sync::atomic::Ordering::Relaxed);
                                state_inspect.inspect_cursor_active.store(new_val, std::sync::atomic::Ordering::Relaxed);
                                bridge_inspect.notify();
                                cx.notify();
                            }))
                            .child(if inspect_active { "🔍 Inspecting" } else { "🔍 Inspect" })
                    )
                    // Horizontal Tab Buttons
                    .children(tabs.into_iter().map(move |(tab, label)| {
                        let is_active = tab == active_tab;
                        let st = state_tabs.clone();
                        div()
                            .id(ElementId::NamedInteger(SharedString::new_static("dev_tab"), tab as u64))
                            .px(px(12.0))
                            .h(px(35.0))
                            .flex()
                            .flex_row()
                            .items_center()
                            .cursor_pointer()
                            .border_b_2()
                            .border_color(if is_active { rgb(0x89b4fa) } else { rgba(0x00000000) })
                            .text_color(if is_active { rgb(0xcdd6f4) } else { rgb(0x6c7086) })
                            .text_size(px(12.0))
                            .font_weight(if is_active { gpui::FontWeight::BOLD } else { gpui::FontWeight::NORMAL })
                            .on_click(cx.listener(move |_this, _event, _window, cx| {
                                *st.active_tab.write() = tab;
                                cx.notify();
                            }))
                            .child(label)
                    }))
            )
            // Right Side Telemetry Badges
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(10.0))
                    .px(px(6.0))
                    .text_size(px(11.0))
                    .text_color(rgb(0x6c7086))
                    .child(div().child(format!("{node_count} nodes")))
                    .child(div().text_color(rgb(0xa6e3a1)).child(format!("{fps:.0} FPS")))
            )
    }

    // ── Tab 1: Elements ─────────────────────────────────────────────────────────
    fn render_elements_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tree_guard = self.state.element_tree.read();
        let selected_path = self.state.selected_path.read().clone();
        let sub_tab = *self.state.elements_sub_tab.read();

        let selected_node = selected_path
            .as_ref()
            .and_then(|p| tree_guard.as_ref().and_then(|t| t.find_by_path(p)));

        let state_sub = self.state.clone();

        let mut tree_rows: Vec<AnyElement> = Vec::new();
        if let Some(ref root) = *tree_guard {
            self.collect_tree_rows(root, selected_path.as_deref(), 0, &mut tree_rows, cx);
        }

        div()
            .size_full()
            .flex()
            .flex_row()
            .overflow_hidden()
            // Left Pane: DOM Element Hierarchy Tree
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .border_r_1()
                    .border_color(rgb(0x313244))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        div()
                            .h(px(28.0))
                            .w_full()
                            .bg(rgb(0x11111b))
                            .border_b_1()
                            .border_color(rgb(0x313244))
                            .px(px(8.0))
                            .flex()
                            .flex_row()
                            .items_center()
                            .text_size(px(11.0))
                            .text_color(rgb(0xa6adc8))
                            .child("Elements Hierarchy")
                    )
                    .child(
                        div()
                            .id("elements_tree_scroll")
                            .flex()
                            .flex_col()
                            .flex_1()
                            .w_full()
                            .overflow_scroll()
                            .py(px(4.0))
                            .children(if !tree_rows.is_empty() {
                                tree_rows
                            } else {
                                vec![div()
                                    .p(px(12.0))
                                    .text_color(rgb(0x6c7086))
                                    .child("<No active component rendered>")
                                    .into_any_element()]
                            })
                    )
            )
            // Right Pane: Inspector (Styles & Box Model)
            .child(
                div()
                    .w(px(380.0))
                    .h_full()
                    .flex()
                    .flex_col()
                    .bg(rgb(0x181825))
                    .overflow_hidden()
                    // Horizontal Inspector Tabs
                    .child(
                        div()
                            .h(px(28.0))
                            .w_full()
                            .bg(rgb(0x11111b))
                            .border_b_1()
                            .border_color(rgb(0x313244))
                            .flex()
                            .flex_row()
                            .items_center()
                            .px(px(6.0))
                            .gap(px(4.0))
                            .child(
                                div()
                                    .id("subtab_styles")
                                    .px(px(10.0))
                                    .h(px(27.0))
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .cursor_pointer()
                                    .border_b_2()
                                    .border_color(if sub_tab == ElementsSubTab::Styles { rgb(0x89b4fa) } else { rgba(0x00000000) })
                                    .text_color(if sub_tab == ElementsSubTab::Styles { rgb(0xcdd6f4) } else { rgb(0x6c7086) })
                                    .text_size(px(11.0))
                                    .on_click(cx.listener({
                                        let st = state_sub.clone();
                                        move |_this, _event, _window, cx| {
                                            *st.elements_sub_tab.write() = ElementsSubTab::Styles;
                                            cx.notify();
                                        }
                                    }))
                                    .child("Styles")
                            )
                            .child(
                                div()
                                    .id("subtab_boxmodel")
                                    .px(px(10.0))
                                    .h(px(27.0))
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .cursor_pointer()
                                    .border_b_2()
                                    .border_color(if sub_tab == ElementsSubTab::BoxModel { rgb(0x89b4fa) } else { rgba(0x00000000) })
                                    .text_color(if sub_tab == ElementsSubTab::BoxModel { rgb(0xcdd6f4) } else { rgb(0x6c7086) })
                                    .text_size(px(11.0))
                                    .on_click(cx.listener({
                                        let st = state_sub.clone();
                                        move |_this, _event, _window, cx| {
                                            *st.elements_sub_tab.write() = ElementsSubTab::BoxModel;
                                            cx.notify();
                                        }
                                    }))
                                    .child("Box Model")
                            )
                    )
                    // Sub-tab Content Area
                    .child(
                        div()
                            .id("styles_scroll")
                            .flex()
                            .flex_col()
                            .flex_1()
                            .w_full()
                            .overflow_scroll()
                            .p(px(12.0))
                            .child(if let Some(node) = selected_node {
                                match sub_tab {
                                    ElementsSubTab::Styles => self.render_node_styles(node).into_any_element(),
                                    ElementsSubTab::BoxModel => self.render_box_model_diagram(node).into_any_element(),
                                }
                            } else {
                                div()
                                    .text_size(px(12.0))
                                    .text_color(rgb(0x6c7086))
                                    .child("Select an element in the hierarchy tree to inspect its computed styles.")
                                    .into_any_element()
                            })
                    )
            )
    }

    fn collect_tree_rows(
        &self,
        node: &crate::devtools::state::ElementTreeNode,
        selected_path: Option<&[usize]>,
        depth: usize,
        rows: &mut Vec<AnyElement>,
        cx: &mut Context<Self>,
    ) {
        let is_selected = selected_path == Some(&node.path);
        let path_clone = node.path.clone();
        let st_select = self.state.clone();
        let bridge_select = self.runtime.bridge().clone();

        let has_children = !node.children.is_empty();
        let is_collapsed = self.state.is_collapsed(&node.path);

        let tag_color = rgb(0xf38ba8);      // Pink/red for tags
        let attr_key_color = rgb(0xfab387); // Orange for attribute keys
        let attr_val_color = rgb(0xa6e3a1); // Green for attribute values
        let text_val_color = rgb(0x94e2d5); // Teal for text previews
        let bracket_color = rgb(0x89dceb);  // Cyan for brackets

        let indent_px = px(8.0 + depth as f32 * 14.0);

        let mut row = div()
            .id(ElementId::NamedInteger(SharedString::new_static("tree_row"), hash_path(&node.path)))
            .flex()
            .flex_row()
            .items_center()
            .h(px(22.0))
            .w_full()
            .flex_shrink_0()
            .pl(indent_px)
            .pr(px(8.0))
            .rounded(px(2.0))
            .cursor_pointer()
            .gap(px(4.0))
            .text_size(px(12.0));

        if is_selected {
            row = row.bg(rgb(0x1e3a5f)).border_l_2().border_color(rgb(0x3b82f6));
        } else {
            row = row.hover(|s| s.bg(rgba(0xffffff0a)));
        }

        // Disclosure Arrow
        if has_children {
            let path_toggle = node.path.clone();
            let st_toggle = self.state.clone();
            let arrow_str = if is_collapsed { "▶" } else { "▼" };
            row = row.child(
                div()
                    .id(ElementId::NamedInteger(SharedString::new_static("arrow"), hash_path(&node.path)))
                    .w(px(12.0))
                    .text_size(px(8.0))
                    .text_color(rgb(0x6c7086))
                    .cursor_pointer()
                    .on_click(cx.listener(move |_this, _event, _window, cx| {
                        st_toggle.toggle_collapsed(&path_toggle);
                        cx.notify();
                    }))
                    .child(arrow_str)
            );
        } else {
            row = row.child(div().w(px(12.0)));
        }

        // Selection & Hover Event Handlers
        let path_hover = node.path.clone();
        row = row
            .on_click(cx.listener(move |_this, _event, _window, cx| {
                *st_select.selected_path.write() = Some(path_clone.clone());
                bridge_select.notify();
                cx.notify();
            }))
            .on_hover({
                let st = self.state.clone();
                let br = self.runtime.bridge().clone();
                move |hovered, _window, _cx| {
                    if *hovered {
                        *st.hovered_path.write() = Some(path_hover.clone());
                    } else {
                        let mut guard = st.hovered_path.write();
                        if guard.as_ref() == Some(&path_hover) {
                            *guard = None;
                        }
                    }
                    br.notify();
                }
            });

        // Opening Tag
        row = row.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .child(div().text_color(bracket_color).child("<"))
                .child(div().text_color(tag_color).font_weight(gpui::FontWeight::BOLD).child(node.tag.clone()))
        );

        // ID Attribute
        if let Some(ref id) = node.id {
            row = row.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .child(div().text_color(attr_key_color).child("id="))
                    .child(div().text_color(attr_val_color).child(format!("\"{id}\"")))
            );
        }

        // Other Extracted Attributes
        for (k, v) in &node.attrs {
            row = row.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .child(div().text_color(attr_key_color).child(format!("{k}=")))
                    .child(div().text_color(attr_val_color).child(format!("\"{v}\"")))
            );
        }

        // Text preview or Closing
        if let Some(ref text) = node.text_preview {
            row = row.child(div().text_color(bracket_color).child(">"));
            row = row.child(div().text_color(text_val_color).child(format!("\"{text}\"")));
            row = row.child(div().text_color(bracket_color).child(format!("</{}>", node.tag)));
        } else if has_children && is_collapsed {
            row = row.child(div().text_color(bracket_color).child(">"));
            row = row.child(div().text_color(rgb(0x6c7086)).child(format!("... </{}>", node.tag)));
        } else {
            row = row.child(div().text_color(bracket_color).child(">"));
        }

        rows.push(row.into_any_element());

        if has_children && !is_collapsed {
            for child in &node.children {
                self.collect_tree_rows(child, selected_path, depth + 1, rows, cx);
            }

            // Closing Tag Row
            let closing_row = div()
                .flex()
                .flex_row()
                .items_center()
                .h(px(20.0))
                .w_full()
                .flex_shrink_0()
                .pl(indent_px + px(12.0))
                .text_size(px(12.0))
                .text_color(bracket_color)
                .child(format!("</{}>", node.tag));
            rows.push(closing_row.into_any_element());
        }
    }

    fn render_node_styles(&self, node: &crate::devtools::state::ElementTreeNode) -> impl IntoElement {
        let s = &node.style;
        let mut props: Vec<(&str, String)> = Vec::new();

        if let Some(pos) = s.position {
            props.push(("position", format!("{pos:?}").to_lowercase()));
        }
        if let Some(d) = s.display {
            props.push(("display", format!("{d:?}").to_lowercase()));
        }
        if let Some(flex) = s.flex {
            props.push(("flex-direction", format!("{flex:?}").to_lowercase()));
        }
        if let Some(items) = s.items {
            props.push(("align-items", format!("{items:?}").to_lowercase()));
        }
        if let Some(justify) = s.justify {
            props.push(("justify-content", format!("{justify:?}").to_lowercase()));
        }
        if let Some(gap) = s.gap {
            props.push(("gap", format!("{gap:.1}px")));
        }
        if let Some(w) = s.width {
            props.push(("width", format!("{w:?}")));
        }
        if let Some(h) = s.height {
            props.push(("height", format!("{h:?}")));
        }
        if let Some(ref bg) = s.background {
            props.push(("background", format!("{bg:?}")));
        }
        if let Some(ref c) = s.text_color {
            props.push(("color", format!("{c:?}")));
        }
        if let Some(ref ff) = s.font_family {
            props.push(("font-family", ff.clone()));
        }
        if let Some(op) = s.opacity {
            props.push(("opacity", format!("{op:.2}")));
        }
        if let Some(r) = s.corner_radius {
            props.push(("border-radius", format!("{r:.1}px")));
        }
        if s.border_widths.top.unwrap_or(0.0) > 0.0 || s.border_widths.bottom.unwrap_or(0.0) > 0.0 {
            props.push(("border-width", format!("{:.1}px", s.border_widths.top.unwrap_or(0.0))));
        }

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(rgb(0x89b4fa))
                    .pb(px(4.0))
                    .child(format!("Matched Styles for <{}>", node.tag))
            )
            .children(props.into_iter().map(|(key, val)| {
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .py(px(3.0))
                    .border_b_1()
                    .border_color(rgb(0x313244))
                    .text_size(px(12.0))
                    .child(div().text_color(rgb(0xa6adc8)).child(key))
                    .child(div().text_color(rgb(0xf9e2af)).child(val))
            }))
    }

    fn render_box_model_diagram(&self, node: &crate::devtools::state::ElementTreeNode) -> impl IntoElement {
        let bounds_map = self.state.node_bounds.read();
        let bounds = bounds_map.get(&node.path).copied().unwrap_or_default();
        let m = crate::devtools::state::BoxModelMetrics::from_style_and_bounds(&node.style, bounds);

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(12.0))
            .child(div().font_weight(gpui::FontWeight::BOLD).text_color(rgb(0x89b4fa)).child("Box Model Diagram"))
            // Outer Margin Box (Orange #f6b26b)
            .child(
                div()
                    .p(px(8.0))
                    .rounded(px(6.0))
                    .bg(rgba(0xf6b26b44))
                    .border_1()
                    .border_color(rgb(0xf6b26b))
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(div().text_size(px(10.0)).text_color(rgb(0xf6b26b)).child(format!("margin: {:.0}", m.margin_top)))
                    // Middle Border Box (Yellow #ffe599)
                    .child(
                        div()
                            .p(px(8.0))
                            .rounded(px(4.0))
                            .bg(rgba(0xffe59944))
                            .border_1()
                            .border_color(rgb(0xffe599))
                            .flex()
                            .flex_col()
                            .items_center()
                            .child(div().text_size(px(10.0)).text_color(rgb(0xffe599)).child(format!("border: {:.0}", m.border_top)))
                            // Inner Padding Box (Green #b6d7a8)
                            .child(
                                div()
                                    .p(px(8.0))
                                    .rounded(px(3.0))
                                    .bg(rgba(0xb6d7a855))
                                    .border_1()
                                    .border_color(rgb(0xb6d7a8))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .child(div().text_size(px(10.0)).text_color(rgb(0xb6d7a8)).child(format!("padding: {:.0}", m.padding_top)))
                                    // Content Box (Blue #9fc5e8)
                                    .child(
                                        div()
                                            .px(px(16.0))
                                            .py(px(8.0))
                                            .rounded(px(2.0))
                                            .bg(rgb(0x3b82f6))
                                            .text_color(rgb(0x11111b))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .child(format!("{:.0} × {:.0}", m.content_width, m.content_height))
                                    )
                                    .child(div().text_size(px(10.0)).text_color(rgb(0xb6d7a8)).child(format!("{:.0}", m.padding_bottom)))
                            )
                            .child(div().text_size(px(10.0)).text_color(rgb(0xffe599)).child(format!("{:.0}", m.border_bottom)))
                    )
                    .child(div().text_size(px(10.0)).text_color(rgb(0xf6b26b)).child(format!("{:.0}", m.margin_bottom)))
            )
    }

    // ── Tab 2: Network (Fetch, WebSocket, WebRTC) ───────────────────────────────
    fn render_network_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entries = self.state.network_entries.read().clone();
        let selected_id = *self.state.selected_network_id.read();
        let filter = *self.state.network_filter.read();

        let filtered_entries: Vec<_> = entries
            .into_iter()
            .filter(|e| filter.is_none() || Some(e.entry_type) == filter)
            .collect();

        let selected_entry = selected_id.and_then(|id| {
            filtered_entries.iter().find(|e| e.id == id).cloned()
        });

        let st_clear = self.state.clone();
        let st_filter = self.state.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            // Horizontal Filter Toolbar Across Top
            .child(
                div()
                    .h(px(32.0))
                    .w_full()
                    .bg(rgb(0x11111b))
                    .border_b_1()
                    .border_color(rgb(0x313244))
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px(px(8.0))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .id("net_filter_all")
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .rounded(px(3.0))
                                    .cursor_pointer()
                                    .bg(if filter.is_none() { rgb(0x313244) } else { rgba(0x00000000) })
                                    .text_color(rgb(0xcdd6f4))
                                    .text_size(px(11.0))
                                    .on_click(cx.listener({
                                        let st = st_filter.clone();
                                        move |_this, _event, _window, cx| {
                                            *st.network_filter.write() = None;
                                            cx.notify();
                                        }
                                    }))
                                    .child("All")
                            )
                            .child(
                                div()
                                    .id("net_filter_fetch")
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .rounded(px(3.0))
                                    .cursor_pointer()
                                    .bg(if filter == Some(NetworkType::Fetch) { rgb(0x313244) } else { rgba(0x00000000) })
                                    .text_color(rgb(0x89b4fa))
                                    .text_size(px(11.0))
                                    .on_click(cx.listener({
                                        let st = st_filter.clone();
                                        move |_this, _event, _window, cx| {
                                            *st.network_filter.write() = Some(NetworkType::Fetch);
                                            cx.notify();
                                        }
                                    }))
                                    .child("Fetch/XHR")
                            )
                            .child(
                                div()
                                    .id("net_filter_ws")
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .rounded(px(3.0))
                                    .cursor_pointer()
                                    .bg(if filter == Some(NetworkType::WebSocket) { rgb(0x313244) } else { rgba(0x00000000) })
                                    .text_color(rgb(0xa6e3a1))
                                    .text_size(px(11.0))
                                    .on_click(cx.listener({
                                        let st = st_filter.clone();
                                        move |_this, _event, _window, cx| {
                                            *st.network_filter.write() = Some(NetworkType::WebSocket);
                                            cx.notify();
                                        }
                                    }))
                                    .child("WebSocket")
                            )
                            .child(
                                div()
                                    .id("net_filter_webrtc")
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .rounded(px(3.0))
                                    .cursor_pointer()
                                    .bg(if filter == Some(NetworkType::WebRtc) { rgb(0x313244) } else { rgba(0x00000000) })
                                    .text_color(rgb(0xf9e2af))
                                    .text_size(px(11.0))
                                    .on_click(cx.listener({
                                        let st = st_filter.clone();
                                        move |_this, _event, _window, cx| {
                                            *st.network_filter.write() = Some(NetworkType::WebRtc);
                                            cx.notify();
                                        }
                                    }))
                                    .child("WebRTC")
                            )
                    )
                    .child(
                        div()
                            .id("net_clear_btn")
                            .px(px(8.0))
                            .py(px(3.0))
                            .rounded(px(3.0))
                            .cursor_pointer()
                            .bg(rgb(0x313244))
                            .text_color(rgb(0xa6adc8))
                            .text_size(px(11.0))
                            .on_click(cx.listener(move |_this, _event, _window, cx| {
                                st_clear.network_entries.write().clear();
                                *st_clear.selected_network_id.write() = None;
                                cx.notify();
                            }))
                            .child("Clear")
                    )
            )
            // Table and Detail Split
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .flex()
                    .flex_row()
                    .overflow_hidden()
                    // Table Area
                    .child(
                        div()
                            .id("net_table_scroll")
                            .flex_1()
                            .h_full()
                            .border_r_1()
                            .border_color(rgb(0x313244))
                            .flex()
                            .flex_col()
                            .overflow_scroll()
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .bg(rgb(0x181825))
                                    .py(px(4.0))
                                    .px(px(8.0))
                                    .text_size(px(11.0))
                                    .text_color(rgb(0x6c7086))
                                    .border_b_1()
                                    .border_color(rgb(0x313244))
                                    .child(div().w(px(220.0)).child("Name / URL"))
                                    .child(div().w(px(60.0)).child("Status"))
                                    .child(div().w(px(60.0)).child("Method"))
                                    .child(div().w(px(70.0)).child("Type"))
                                    .child(div().w(px(60.0)).child("Time"))
                            )
                            .children(filtered_entries.iter().map(|entry| {
                                let id = entry.id;
                                let is_sel = selected_id == Some(id);
                                let st_sel = self.state.clone();
                                let status_color = match entry.status {
                                    Some(200..=299) | Some(101) => rgb(0xa6e3a1),
                                    Some(400..=599) => rgb(0xf38ba8),
                                    _ => rgb(0xf9e2af),
                                };

                                div()
                                    .id(ElementId::NamedInteger(SharedString::new_static("net_row"), id))
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .py(px(4.0))
                                    .px(px(8.0))
                                    .border_b_1()
                                    .border_color(rgb(0x1e1e2e))
                                    .cursor_pointer()
                                    .bg(if is_sel { rgb(0x1f4068) } else { rgba(0x00000000) })
                                    .on_click(cx.listener(move |_this, _event, _window, cx| {
                                        *st_sel.selected_network_id.write() = Some(id);
                                        cx.notify();
                                    }))
                                    .child(div().w(px(220.0)).text_color(rgb(0xcdd6f4)).child(entry.url.clone()))
                                    .child(div().w(px(60.0)).text_color(status_color).child(entry.status.map(|s| s.to_string()).unwrap_or_else(|| "---".to_string())))
                                    .child(div().w(px(60.0)).text_color(rgb(0xf9e2af)).child(entry.method.clone()))
                                    .child(div().w(px(70.0)).text_color(rgb(0xa6adc8)).child(format!("{:?}", entry.entry_type)))
                                    .child(div().w(px(60.0)).text_color(rgb(0x6c7086)).child(entry.duration_ms.map(|d| format!("{d:.0}ms")).unwrap_or_else(|| "-".to_string())))
                            }))
                    )
                    // Request Detail Drawer
                    .child(
                        div()
                            .id("net_detail_scroll")
                            .w(px(380.0))
                            .h_full()
                            .flex()
                            .flex_col()
                            .overflow_scroll()
                            .p(px(12.0))
                            .child(if let Some(entry) = selected_entry {
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(10.0))
                                    .child(div().font_weight(gpui::FontWeight::BOLD).text_color(rgb(0x89b4fa)).child(format!("{} {}", entry.method, entry.url)))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.0))
                                            .child(div().text_color(rgb(0xa6adc8)).font_weight(gpui::FontWeight::BOLD).child("Response Headers:"))
                                            .children(entry.response_headers.into_iter().map(|(k, v)| {
                                                div().text_size(px(11.0)).child(format!("{k}: {v}"))
                                            }))
                                    )
                                    .child(if !entry.ws_frames.is_empty() {
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.0))
                                            .child(div().text_color(rgb(0xa6e3a1)).font_weight(gpui::FontWeight::BOLD).child("WebSocket Frames:"))
                                            .children(entry.ws_frames.into_iter().map(|f| {
                                                let arrow = if f.is_outgoing { "↑ SENT" } else { "↓ RECV" };
                                                let color = if f.is_outgoing { rgb(0x89b4fa) } else { rgb(0xa6e3a1) };
                                                div().text_size(px(11.0)).text_color(color).child(format!("{} [{}] {}", arrow, f.timestamp, f.payload))
                                            }))
                                            .into_any_element()
                                    } else {
                                        div().into_any_element()
                                    })
                                    .child(if let Some(stats) = entry.webrtc_stats {
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.0))
                                            .child(div().text_color(rgb(0xf9e2af)).font_weight(gpui::FontWeight::BOLD).child("WebRTC Connection Stats:"))
                                            .child(div().text_size(px(11.0)).child(stats))
                                            .into_any_element()
                                    } else {
                                        div().into_any_element()
                                    })
                                    .child(if let Some(body) = entry.response_body {
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.0))
                                            .child(div().text_color(rgb(0xa6adc8)).font_weight(gpui::FontWeight::BOLD).child("Response Body:"))
                                            .child(div().p(px(8.0)).rounded(px(4.0)).bg(rgb(0x11111b)).text_size(px(11.0)).child(body))
                                            .into_any_element()
                                    } else {
                                        div().into_any_element()
                                    })
                                    .into_any_element()
                            } else {
                                div().text_color(rgb(0x6c7086)).child("Select a request to view headers and details.").into_any_element()
                            })
                    )
            )
    }

    // ── Tab 3: Storage (Horizontal Tabs: LocalStorage / Databases / Signals) ────
    fn render_storage_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let sub = *self.state.storage_sub_tab.read();
        let st_sub = self.state.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            // Horizontal Storage Tab Bar Across Top
            .child(
                div()
                    .h(px(32.0))
                    .w_full()
                    .bg(rgb(0x11111b))
                    .border_b_1()
                    .border_color(rgb(0x313244))
                    .flex()
                    .flex_row()
                    .items_center()
                    .px(px(8.0))
                    .gap(px(4.0))
                    .child(
                        div()
                            .id("storage_tab_ls")
                            .px(px(12.0))
                            .h(px(31.0))
                            .flex()
                            .flex_row()
                            .items_center()
                            .cursor_pointer()
                            .border_b_2()
                            .border_color(if sub == StorageSubTab::LocalStorage { rgb(0x89b4fa) } else { rgba(0x00000000) })
                            .text_color(if sub == StorageSubTab::LocalStorage { rgb(0xcdd6f4) } else { rgb(0x6c7086) })
                            .text_size(px(11.0))
                            .font_weight(if sub == StorageSubTab::LocalStorage { gpui::FontWeight::BOLD } else { gpui::FontWeight::NORMAL })
                            .on_click(cx.listener({
                                let st = st_sub.clone();
                                move |_this, _event, _window, cx| {
                                    *st.storage_sub_tab.write() = StorageSubTab::LocalStorage;
                                    cx.notify();
                                }
                            }))
                            .child("📁 LocalStorage")
                    )
                    .child(
                        div()
                            .id("storage_tab_db")
                            .px(px(12.0))
                            .h(px(31.0))
                            .flex()
                            .flex_row()
                            .items_center()
                            .cursor_pointer()
                            .border_b_2()
                            .border_color(if sub == StorageSubTab::Databases { rgb(0x89b4fa) } else { rgba(0x00000000) })
                            .text_color(if sub == StorageSubTab::Databases { rgb(0xcdd6f4) } else { rgb(0x6c7086) })
                            .text_size(px(11.0))
                            .font_weight(if sub == StorageSubTab::Databases { gpui::FontWeight::BOLD } else { gpui::FontWeight::NORMAL })
                            .on_click(cx.listener({
                                let st = st_sub.clone();
                                move |_this, _event, _window, cx| {
                                    *st.storage_sub_tab.write() = StorageSubTab::Databases;
                                    cx.notify();
                                }
                            }))
                            .child("🗄️ SQLite Databases")
                    )
                    .child(
                        div()
                            .id("storage_tab_sig")
                            .px(px(12.0))
                            .h(px(31.0))
                            .flex()
                            .flex_row()
                            .items_center()
                            .cursor_pointer()
                            .border_b_2()
                            .border_color(if sub == StorageSubTab::Signals { rgb(0x89b4fa) } else { rgba(0x00000000) })
                            .text_color(if sub == StorageSubTab::Signals { rgb(0xcdd6f4) } else { rgb(0x6c7086) })
                            .text_size(px(11.0))
                            .font_weight(if sub == StorageSubTab::Signals { gpui::FontWeight::BOLD } else { gpui::FontWeight::NORMAL })
                            .on_click(cx.listener({
                                let st = st_sub.clone();
                                move |_this, _event, _window, cx| {
                                    *st.storage_sub_tab.write() = StorageSubTab::Signals;
                                    cx.notify();
                                }
                            }))
                            .child("⚡ Reactive Signals")
                    )
            )
            // Full Width Workspace Below
            .child(
                div()
                    .id("storage_scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .w_full()
                    .overflow_scroll()
                    .p(px(14.0))
                    .child(match sub {
                        StorageSubTab::LocalStorage => self.render_localstorage_view(cx).into_any_element(),
                        StorageSubTab::Databases => self.render_sqlite_view(cx).into_any_element(),
                        StorageSubTab::Signals => self.render_signals_view().into_any_element(),
                    })
            )
    }

    fn render_localstorage_view(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let items: Vec<(String, String)> = {
            let lua_arc = self.runtime.lua();
            let lua = lua_arc.lock();
            if let Ok(storage_tbl) = lua.globals().get::<mlua::Table>("storage") {
                if let Ok(all_fn) = storage_tbl.get::<mlua::Function>("all") {
                    if let Ok(mlua::Value::Table(t)) = all_fn.call::<mlua::Value>(()) {
                        let mut res = Vec::new();
                        for pair in t.pairs::<String, mlua::Value>() {
                            if let Ok((k, v)) = pair {
                                let val_str = match v {
                                    mlua::Value::String(s) => s.to_str().unwrap_or_default().to_string(),
                                    _ => format!("{v:?}"),
                                };
                                res.push((k, val_str));
                            }
                        }
                        res
                    } else { Vec::new() }
                } else { Vec::new() }
            } else { Vec::new() }
        };

        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(div().font_weight(gpui::FontWeight::BOLD).text_color(rgb(0x89b4fa)).child("LocalStorage Key-Value Store"))
            .children(items.into_iter().map(|(k, v)| {
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .p(px(8.0))
                    .bg(rgb(0x11111b))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(rgb(0x313244))
                    .child(div().text_color(rgb(0xa6adc8)).font_weight(gpui::FontWeight::BOLD).child(k))
                    .child(div().text_color(rgb(0xa6e3a1)).child(v))
            }))
    }

    fn render_sqlite_view(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.state.sql_query.read().clone();

        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(div().font_weight(gpui::FontWeight::BOLD).text_color(rgb(0x89b4fa)).child("SQLite Database Explorer"))
            .child(
                div()
                    .p(px(8.0))
                    .bg(rgb(0x11111b))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(rgb(0x313244))
                    .text_color(rgb(0xf9e2af))
                    .text_size(px(12.0))
                    .child(query)
            )
            .child(div().text_color(rgb(0x6c7086)).text_size(px(12.0)).child("Open databases via db.open(path) to inspect tables and run queries."))
    }

    fn render_signals_view(&self) -> impl IntoElement {
        let store = self.runtime.store();
        let keys = store.keys();

        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(div().font_weight(gpui::FontWeight::BOLD).text_color(rgb(0x89b4fa)).child("Active Reactive Signals (Hot-Reload Preserved)"))
            .children(keys.into_iter().map(|k| {
                let val = store.get(&k).unwrap_or(crate::reactive::store::StoredValue::Nil);
                let val_str = match val {
                    crate::reactive::store::StoredValue::Integer(i) => i.to_string(),
                    crate::reactive::store::StoredValue::Number(n) => n.to_string(),
                    crate::reactive::store::StoredValue::String(s) => s,
                    crate::reactive::store::StoredValue::Boolean(b) => b.to_string(),
                    crate::reactive::store::StoredValue::Json(j) => j.to_string(),
                    crate::reactive::store::StoredValue::Nil => "nil".to_string(),
                };

                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .p(px(8.0))
                    .bg(rgb(0x11111b))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(rgb(0x313244))
                    .child(div().text_color(rgb(0x89b4fa)).font_weight(gpui::FontWeight::BOLD).child(k))
                    .child(div().text_color(rgb(0xa6e3a1)).child(val_str))
            }))
    }

    // ── Tab 4: Console ──────────────────────────────────────────────────────────
    fn render_console_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let logs = self.state.console_entries.read().clone();
        let st_clear = self.state.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .h(px(32.0))
                    .w_full()
                    .bg(rgb(0x11111b))
                    .border_b_1()
                    .border_color(rgb(0x313244))
                    .flex()
                    .flex_row()
                    .justify_between()
                    .items_center()
                    .px(px(8.0))
                    .child(div().text_size(px(11.0)).font_weight(gpui::FontWeight::BOLD).text_color(rgb(0xa6adc8)).child("Console Output & REPL"))
                    .child(
                        div()
                            .id("console_clear_btn")
                            .px(px(8.0))
                            .py(px(3.0))
                            .rounded(px(3.0))
                            .cursor_pointer()
                            .bg(rgb(0x313244))
                            .text_color(rgb(0xcdd6f4))
                            .text_size(px(11.0))
                            .on_click(cx.listener(move |_this, _event, _window, cx| {
                                st_clear.console_entries.write().clear();
                                cx.notify();
                            }))
                            .child("Clear")
                    )
            )
            .child(
                div()
                    .id("console_scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .w_full()
                    .overflow_scroll()
                    .p(px(10.0))
                    .gap(px(4.0))
                    .children(logs.into_iter().map(|log| {
                        let (badge_col, label) = match log.level {
                            LogLevel::Info => (rgb(0x89b4fa), "[INFO]"),
                            LogLevel::Warn => (rgb(0xf9e2af), "[WARN]"),
                            LogLevel::Error => (rgb(0xf38ba8), "[ERROR]"),
                            LogLevel::Debug => (rgb(0x6c7086), "[DEBUG]"),
                        };

                        div()
                            .flex()
                            .flex_row()
                            .gap(px(8.0))
                            .text_size(px(11.0))
                            .child(div().text_color(rgb(0x6c7086)).child(log.time_str))
                            .child(div().text_color(badge_col).font_weight(gpui::FontWeight::BOLD).child(label))
                            .child(div().text_color(rgb(0xcdd6f4)).child(log.message))
                    }))
            )
    }

    // ── Tab 5: Performance ──────────────────────────────────────────────────────
    fn render_performance_tab(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        let fps = *self.state.fps.read();
        let frame_ms = *self.state.frame_time_ms.read();
        let node_count = self.state.node_bounds.read().len();

        let lua_mem_kb: usize = {
            let lua_arc = self.runtime.lua();
            let lua = lua_arc.lock();
            lua.used_memory() / 1024
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .p(px(16.0))
            .gap(px(14.0))
            .child(div().font_weight(gpui::FontWeight::BOLD).text_color(rgb(0x89b4fa)).text_size(px(14.0)).child("Performance & Engine Telemetry"))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(12.0))
                    .child(self.render_metric_card("Frame Rate", format!("{fps:.0} FPS"), rgb(0xa6e3a1)))
                    .child(self.render_metric_card("Frame Time", format!("{frame_ms:.1} ms"), rgb(0x89b4fa)))
                    .child(self.render_metric_card("DOM Node Count", node_count.to_string(), rgb(0xf9e2af)))
                    .child(self.render_metric_card("Lua Memory", format!("{lua_mem_kb} KB"), rgb(0xcba6f7)))
            )
    }

    fn render_metric_card(&self, title: &'static str, value: String, color: gpui::Rgba) -> impl IntoElement {
        div()
            .p(px(12.0))
            .rounded(px(6.0))
            .bg(rgb(0x11111b))
            .border_1()
            .border_color(rgb(0x313244))
            .w(px(160.0))
            .flex()
            .flex_col()
            .child(div().text_size(px(11.0)).text_color(rgb(0xa6adc8)).child(title))
            .child(div().text_size(px(18.0)).font_weight(gpui::FontWeight::BOLD).text_color(color).child(value))
    }
}

fn hash_path(p: &[usize]) -> u64 {
    let mut h = 0x12345678u64;
    for &x in p {
        h = h.wrapping_mul(31).wrapping_add(x as u64);
    }
    h
}
