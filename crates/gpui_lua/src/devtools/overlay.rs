use crate::devtools::state::{BoxModelMetrics, DevToolsState};
use crate::dsl::node::StyleProps;
use gpui::{canvas, fill, outline, px, rgba, Bounds, IntoElement, Point, Size, Styled};
use std::sync::Arc;

pub fn render_devtools_overlay(
    devtools: Arc<DevToolsState>,
) -> impl IntoElement {
    let devtools_clone = devtools.clone();

    canvas(
        move |_bounds, _window, _cx| (),
        move |_canvas_bounds, (), window, _cx| {
            let active_path = devtools_clone.hovered_path.read().clone()
                .or_else(|| devtools_clone.selected_path.read().clone());

            let Some(path) = active_path else {
                return;
            };

            let bounds_map = devtools_clone.node_bounds.read();
            let Some(&node_bounds) = bounds_map.get(&path) else {
                return;
            };

            let tree_guard = devtools_clone.element_tree.read();
            let style = if let Some(ref tree) = *tree_guard {
                tree.find_by_path(&path).map(|n| n.style.clone()).unwrap_or_default()
            } else {
                StyleProps::default()
            };

            let tag = if let Some(ref tree) = *tree_guard {
                tree.find_by_path(&path).map(|n| {
                    if let Some(ref id) = n.id {
                        format!("{}#{}", n.tag, id)
                    } else {
                        n.tag.clone()
                    }
                }).unwrap_or_else(|| "element".to_string())
            } else {
                "element".to_string()
            };

            let box_model = BoxModelMetrics::from_style_and_bounds(&style, node_bounds);

            // Coordinates
            let bx = node_bounds.origin.x;
            let by = node_bounds.origin.y;
            let bw = node_bounds.size.width;
            let bh = node_bounds.size.height;

            // 1. Margin Box (Orange #f6b26b66)
            let margin_bounds = Bounds {
                origin: Point::new(bx - px(box_model.margin_left), by - px(box_model.margin_top)),
                size: Size::new(
                    bw + px(box_model.margin_left + box_model.margin_right),
                    bh + px(box_model.margin_top + box_model.margin_bottom),
                ),
            };
            if box_model.margin_top > 0.0 || box_model.margin_bottom > 0.0 || box_model.margin_left > 0.0 || box_model.margin_right > 0.0 {
                window.paint_quad(fill(margin_bounds, rgba(0xf6b26b55)));
            }

            // 2. Border Box (Yellow #ffe59966)
            window.paint_quad(fill(node_bounds, rgba(0xffe59966)));

            // 3. Padding Box (Green #b6d7a877)
            let padding_bounds = Bounds {
                origin: Point::new(bx + px(box_model.border_left), by + px(box_model.border_top)),
                size: Size::new(
                    (bw - px(box_model.border_left + box_model.border_right)).max(px(0.0)),
                    (bh - px(box_model.border_top + box_model.border_bottom)).max(px(0.0)),
                ),
            };
            window.paint_quad(fill(padding_bounds, rgba(0xb6d7a877)));

            // 4. Content Box (Blue #9fc5e899)
            let content_bounds = Bounds {
                origin: Point::new(
                    bx + px(box_model.border_left + box_model.padding_left),
                    by + px(box_model.border_top + box_model.padding_top),
                ),
                size: Size::new(
                    px(box_model.content_width).max(px(0.0)),
                    px(box_model.content_height).max(px(0.0)),
                ),
            };
            window.paint_quad(fill(content_bounds, rgba(0x9fc5e899)));

            // Primary bounding outline (Chromium blue border)
            window.paint_quad(outline(node_bounds, rgba(0x3b82f6ff), gpui::BorderStyle::default()));

            // Floating Tooltip Badge above/below element
            let w_val: f32 = bw.into();
            let h_val: f32 = bh.into();
            let badge_text = format!("{tag} | {w_val:.0} × {h_val:.0} px");
            
            let badge_y = if by > px(30.0) {
                by - px(24.0)
            } else {
                by + bh + px(4.0)
            };

            let badge_w = px(badge_text.len() as f32 * 7.5 + 16.0);
            let badge_h = px(20.0);
            let badge_bounds = Bounds {
                origin: Point::new(bx, badge_y),
                size: Size::new(badge_w, badge_h),
            };

            // Badge background & text
            window.paint_quad(fill(badge_bounds, rgba(0x181825ee)));
            window.paint_quad(outline(badge_bounds, rgba(0x45475aff), gpui::BorderStyle::default()));
        },
    )
    .size_full()
    .absolute()
    .top_0()
    .left_0()
}
