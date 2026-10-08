use crate::devtools::state::{BoxModelMetrics, DevToolsState, ElementTreeNode};
use gpui::{
    canvas, fill, outline, px, rgba, Bounds, IntoElement, PathBuilder, Point, Size, Styled,
    Window,
};
use std::sync::Arc;

pub fn render_devtools_overlay(
    devtools: Arc<DevToolsState>,
) -> impl IntoElement {
    let devtools_clone = devtools.clone();

    canvas(
        move |_bounds, _window, _cx| (),
        move |_canvas_bounds, (), window, cx| {
            if !devtools_clone.enabled.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            let active_path = devtools_clone.hovered_path.read().clone().or_else(|| {
                if devtools_clone.inspect_cursor_active.load(std::sync::atomic::Ordering::Relaxed) {
                    devtools_clone.selected_path.read().clone()
                } else {
                    None
                }
            });

            let Some(path) = active_path else {
                return;
            };

            let bounds_map = devtools_clone.node_bounds.read();
            let Some(&node_bounds) = bounds_map.get(&path) else {
                return;
            };

            let tree_guard = devtools_clone.element_tree.read();
            let node_opt = tree_guard.as_ref().and_then(|t| t.find_by_path(&path));

            let style = node_opt.map(|n| n.style.clone()).unwrap_or_default();
            let tag = node_opt
                .map(|n| {
                    if let Some(ref id) = n.id {
                        format!("{}#{}", n.tag, id)
                    } else {
                        n.tag.clone()
                    }
                })
                .unwrap_or_else(|| "element".to_string());

            let box_model = BoxModelMetrics::from_style_and_bounds(&style, node_bounds);

            // Coordinates
            let bx = node_bounds.origin.x;
            let by = node_bounds.origin.y;
            let bw = node_bounds.size.width;
            let bh = node_bounds.size.height;

            // 1. Margin Box (Orange #f6b26b55)
            let margin_bounds = Bounds {
                origin: Point::new(bx - px(box_model.margin_left), by - px(box_model.margin_top)),
                size: Size::new(
                    bw + px(box_model.margin_left + box_model.margin_right),
                    bh + px(box_model.margin_top + box_model.margin_bottom),
                ),
            };
            if box_model.margin_top > 0.0
                || box_model.margin_bottom > 0.0
                || box_model.margin_left > 0.0
                || box_model.margin_right > 0.0
            {
                window.paint_quad(fill(margin_bounds, rgba(0xf6b26b55)));
            }

            // 2. Border Box (Yellow #ffe59966)
            if box_model.border_top > 0.0
                || box_model.border_bottom > 0.0
                || box_model.border_left > 0.0
                || box_model.border_right > 0.0
            {
                window.paint_quad(fill(node_bounds, rgba(0xffe59966)));
            }

            // 3. Padding Box (Green #b6d7a877)
            let padding_bounds = Bounds {
                origin: Point::new(bx + px(box_model.border_left), by + px(box_model.border_top)),
                size: Size::new(
                    (bw - px(box_model.border_left + box_model.border_right)).max(px(0.0)),
                    (bh - px(box_model.border_top + box_model.border_bottom)).max(px(0.0)),
                ),
            };
            if box_model.padding_top > 0.0
                || box_model.padding_bottom > 0.0
                || box_model.padding_left > 0.0
                || box_model.padding_right > 0.0
            {
                window.paint_quad(fill(padding_bounds, rgba(0xb6d7a877)));
            }

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

            // 5. Flex Layout Shading: Gap & Empty Areas
            let is_flex = style.display == Some(gpui::Display::Flex)
                || style.flex.is_some()
                || tag.starts_with("row")
                || (tag.starts_with("div")
                    && (style.flex.is_some()
                        || style.gap.is_some()
                        || style.items.is_some()
                        || style.justify.is_some()));

            let is_row = matches!(style.flex, Some(gpui::FlexDirection::Row))
                || tag.starts_with("row")
                || (style.flex.is_none() && style.display == Some(gpui::Display::Flex));

            if is_flex {
                shade_flex_container(
                    window,
                    content_bounds,
                    node_opt,
                    &bounds_map,
                    is_row,
                );
            }

            // Also check if parent of active node is a flex container; if so, shade parent's gaps
            if path.len() > 1 {
                let parent_path = &path[..path.len() - 1];
                if let Some(parent_node) = tree_guard.as_ref().and_then(|t| t.find_by_path(parent_path)) {
                    let p_flex = parent_node.style.display == Some(gpui::Display::Flex)
                        || parent_node.style.flex.is_some()
                        || parent_node.tag.starts_with("row")
                        || parent_node.style.gap.is_some();

                    if p_flex {
                        let p_is_row = matches!(parent_node.style.flex, Some(gpui::FlexDirection::Row))
                            || parent_node.tag.starts_with("row");
                        if let Some(&p_bounds) = bounds_map.get(parent_path) {
                            let p_model = BoxModelMetrics::from_style_and_bounds(&parent_node.style, p_bounds);
                            let p_content_bounds = Bounds {
                                origin: Point::new(
                                    p_bounds.origin.x + px(p_model.border_left + p_model.padding_left),
                                    p_bounds.origin.y + px(p_model.border_top + p_model.padding_top),
                                ),
                                size: Size::new(
                                    px(p_model.content_width).max(px(0.0)),
                                    px(p_model.content_height).max(px(0.0)),
                                ),
                            };
                            shade_flex_container(
                                window,
                                p_content_bounds,
                                Some(parent_node),
                                &bounds_map,
                                p_is_row,
                            );
                        }
                    }
                }
            }

            // Primary bounding outline (Chromium blue border)
            window.paint_quad(outline(node_bounds, rgba(0x3b82f6ff), gpui::BorderStyle::default()));

            // Floating Tooltip Badge above/below element
            let w_val: f32 = bw.into();
            let h_val: f32 = bh.into();
            let mut badge_text = format!("{tag} | {w_val:.0} × {h_val:.0} px");
            if is_flex {
                badge_text.push_str(if is_row { " [flex-row]" } else { " [flex-col]" });
            }

            let badge_y = if by > px(30.0) {
                by - px(24.0)
            } else {
                by + bh + px(4.0)
            };

            let char_width = 6.8f32;
            let badge_w = px(badge_text.len() as f32 * char_width + 16.0);
            let badge_h = px(20.0);
            let badge_bounds = Bounds {
                origin: Point::new(bx, badge_y),
                size: Size::new(badge_w, badge_h),
            };

            // Badge background & outline
            window.paint_quad(fill(badge_bounds, rgba(0x181825ee)));
            window.paint_quad(outline(badge_bounds, rgba(0x45475aff), gpui::BorderStyle::default()));

            // Badge text
            let run = gpui::TextRun {
                len: badge_text.len(),
                font: window.text_style().font(),
                color: gpui::rgb_to_hsla(gpui::rgb(0xcdd6f4)),
                ..Default::default()
            };
            let shaped = window.text_system().shape_line(badge_text.into(), px(11.0), &[run], None);
            let _ = shaped.paint(
                Point::new(bx + px(8.0), badge_y + px(3.0)),
                px(14.0),
                gpui::TextAlign::Left,
                None,
                window,
                cx,
            );
        },
    )
    .size_full()
    .absolute()
    .top_0()
    .left_0()
}

fn shade_flex_container(
    window: &mut Window,
    content_bounds: Bounds<gpui::Pixels>,
    node_opt: Option<&ElementTreeNode>,
    bounds_map: &std::collections::HashMap<Vec<usize>, Bounds<gpui::Pixels>>,
    is_row: bool,
) {
    let mut child_bounds: Vec<Bounds<gpui::Pixels>> = Vec::new();
    if let Some(node) = node_opt {
        for child in &node.children {
            if let Some(&cb) = bounds_map.get(&child.path) {
                child_bounds.push(cb);
            }
        }
    }

    if child_bounds.is_empty() {
        if content_bounds.size.width > px(1.0) && content_bounds.size.height > px(1.0) {
            paint_shaded_area(window, content_bounds, "empty");
        }
        return;
    }

    // Sort along primary axis
    if is_row {
        child_bounds.sort_by(|a, b| {
            a.origin.x.partial_cmp(&b.origin.x).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        child_bounds.sort_by(|a, b| {
            a.origin.y.partial_cmp(&b.origin.y).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // 1. Empty area before first child
    if let Some(first) = child_bounds.first() {
        if is_row {
            if first.origin.x > content_bounds.origin.x + px(1.0) {
                let empty_b = Bounds {
                    origin: content_bounds.origin,
                    size: Size::new(first.origin.x - content_bounds.origin.x, content_bounds.size.height),
                };
                paint_shaded_area(window, empty_b, "empty");
            }
        } else if first.origin.y > content_bounds.origin.y + px(1.0) {
            let empty_b = Bounds {
                origin: content_bounds.origin,
                size: Size::new(content_bounds.size.width, first.origin.y - content_bounds.origin.y),
            };
            paint_shaded_area(window, empty_b, "empty");
        }
    }

    // 2. Gaps between consecutive children
    for i in 0..child_bounds.len().saturating_sub(1) {
        let curr = child_bounds[i];
        let next = child_bounds[i + 1];
        if is_row {
            let curr_right = curr.origin.x + curr.size.width;
            if next.origin.x > curr_right + px(0.5) {
                let gap_b = Bounds {
                    origin: Point::new(curr_right, content_bounds.origin.y),
                    size: Size::new(next.origin.x - curr_right, content_bounds.size.height),
                };
                paint_shaded_area(window, gap_b, "gap");
            }
        } else {
            let curr_bottom = curr.origin.y + curr.size.height;
            if next.origin.y > curr_bottom + px(0.5) {
                let gap_b = Bounds {
                    origin: Point::new(content_bounds.origin.x, curr_bottom),
                    size: Size::new(content_bounds.size.width, next.origin.y - curr_bottom),
                };
                paint_shaded_area(window, gap_b, "gap");
            }
        }
    }

    // 3. Empty area after last child
    if let Some(last) = child_bounds.last() {
        if is_row {
            let last_right = last.origin.x + last.size.width;
            let content_right = content_bounds.origin.x + content_bounds.size.width;
            if content_right > last_right + px(1.0) {
                let empty_b = Bounds {
                    origin: Point::new(last_right, content_bounds.origin.y),
                    size: Size::new(content_right - last_right, content_bounds.size.height),
                };
                paint_shaded_area(window, empty_b, "empty");
            }
        } else {
            let last_bottom = last.origin.y + last.size.height;
            let content_bottom = content_bounds.origin.y + content_bounds.size.height;
            if content_bottom > last_bottom + px(1.0) {
                let empty_b = Bounds {
                    origin: Point::new(content_bounds.origin.x, last_bottom),
                    size: Size::new(content_bounds.size.width, content_bottom - last_bottom),
                };
                paint_shaded_area(window, empty_b, "empty");
            }
        }
    }
}

fn paint_shaded_area(window: &mut Window, bounds: Bounds<gpui::Pixels>, area_type: &str) {
    if bounds.size.width <= px(0.5) || bounds.size.height <= px(0.5) {
        return;
    }
    let (base_color, stripe_color, border_color) = if area_type == "gap" {
        (rgba(0xcba6f733), rgba(0xcba6f788), rgba(0xcba6f7cc))
    } else {
        (rgba(0x89b4fa22), rgba(0x89b4fa55), rgba(0x89b4fa88))
    };

    // 1. Base translucent fill
    window.paint_quad(fill(bounds, base_color));

    // 2. Subtle outline
    window.paint_quad(outline(bounds, border_color, gpui::BorderStyle::default()));

    // 3. Diagonal zebra stripes clipped to bounds
    let mask = gpui::ContentMask {
        bounds,
        fade_out: Default::default(),
    };

    window.with_content_mask(Some(mask), |window| {
        let bx: f32 = bounds.origin.x.into();
        let by: f32 = bounds.origin.y.into();
        let bw: f32 = bounds.size.width.into();
        let bh: f32 = bounds.size.height.into();

        let step = 8.0f32;
        let mut builder = PathBuilder::stroke(px(1.5));
        let mut x = -bh;
        while x < bw {
            builder.move_to(Point::new(px(bx + x), px(by)));
            builder.line_to(Point::new(px(bx + x + bh), px(by + bh)));
            x += step;
        }

        if let Ok(path) = builder.build() {
            window.paint_path(path, stripe_color);
        }
    });
}
