use crate::dsl::node::{
    CanvasNode, DivNode, ImgNode, InputNode, Length, LuaNode, ShadowLevel, StyleProps, SvgNode,
    SyncRegistryKey, TextNode, VideoNode,
};
use gpui::{
    black, canvas, div, img, px, relative, svg, Animation, AnimationExt, AnyElement,
    BoxShadow, ColorExt, DefiniteLength, Display, ElementId, FontWeight, InteractiveElement,
    IntoElement, ParentElement, SharedString, StatefulInteractiveElement, Styled, StyledImage,
};
use std::sync::Arc;
use std::time::Duration;

pub trait LuaInvoker: 'static + Send + Sync {
    fn call_key(&self, key: &SyncRegistryKey) -> Result<(), mlua::Error>;
    fn call_key_bool(&self, key: &SyncRegistryKey, arg: bool) -> Result<(), mlua::Error>;
    fn call_key_bounds(
        &self,
        key: &SyncRegistryKey,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) -> Result<(), mlua::Error>;
    fn call_key_paths(&self, key: &SyncRegistryKey, paths: Vec<String>)
        -> Result<(), mlua::Error>;
    fn call_key_mouse(
        &self,
        key: &SyncRegistryKey,
        x: f32,
        y: f32,
        button: &str,
    ) -> Result<(), mlua::Error> {
        let _ = (key, x, y, button);
        Ok(())
    }
    fn call_key_key(
        &self,
        key: &SyncRegistryKey,
        key_str: &str,
        key_char: Option<&str>,
        ctrl: bool,
        alt: bool,
        shift: bool,
        meta: bool,
    ) -> Result<(), mlua::Error> {
        let _ = (key, key_str, key_char, ctrl, alt, shift, meta);
        Ok(())
    }
    fn call_key_scroll(
        &self,
        key: &SyncRegistryKey,
        dx: f32,
        dy: f32,
    ) -> Result<(), mlua::Error> {
        let _ = (key, dx, dy);
        Ok(())
    }
    fn call_key_input(&self, key: &SyncRegistryKey, val: &str) -> Result<(), mlua::Error> {
        let _ = (key, val);
        Ok(())
    }
    fn get_asset(&self, _path: &str) -> Option<std::borrow::Cow<'static, [u8]>> {
        None
    }
    #[cfg(feature = "media")]
    fn get_video_player(&self, _src: &str, _autoplay: bool) -> Option<Arc<media::VideoPlayer>> {
        None
    }
    #[cfg(feature = "media")]
    fn get_audio_player(&self, _src: &str) -> Option<Arc<media::AudioPlayer>> {
        None
    }
}

pub fn convert_node(node: LuaNode, invoker: Option<Arc<dyn LuaInvoker>>) -> AnyElement {
    let mut auto_id = 0u64;
    convert_node_recursive(node, invoker, &mut auto_id, None)
}

fn length_to_definite(len: Length) -> Option<DefiniteLength> {
    match len {
        Length::Px(p) => Some(px(p).into()),
        Length::Percent(pct) => Some(relative(pct / 100.0).into()),
        Length::Full => Some(relative(1.0).into()),
        Length::Auto => None,
    }
}

fn convert_node_recursive(
    node: LuaNode,
    invoker: Option<Arc<dyn LuaInvoker>>,
    auto_id: &mut u64,
    inherited_font_family: Option<&str>,
) -> AnyElement {
    match node {
        LuaNode::Text(text) => convert_text_node(text, inherited_font_family),
        LuaNode::Div(div_node) => convert_div_node(div_node, invoker, auto_id, inherited_font_family),
        LuaNode::Svg(svg_node) => convert_svg_node(svg_node, invoker.as_ref()),
        LuaNode::Img(img_node) => convert_img_node(img_node),
        LuaNode::Canvas(canvas_node) => convert_canvas_node(canvas_node, invoker),
        LuaNode::Video(video_node) => convert_video_node(video_node, invoker),
        LuaNode::Input(input_node) => convert_input_node(input_node, invoker, inherited_font_family),
        LuaNode::Custom(custom_node) => convert_custom_node(custom_node, invoker, auto_id, inherited_font_family),
    }
}

fn convert_custom_node(
    custom: crate::dsl::node::CustomNode,
    invoker: Option<Arc<dyn LuaInvoker>>,
    auto_id: &mut u64,
    inherited_font_family: Option<&str>,
) -> AnyElement {
    let effective_ff = custom.style.font_family.as_deref().or(inherited_font_family);
    let children_elements: Vec<AnyElement> = custom
        .children
        .into_iter()
        .map(|ch| convert_node_recursive(ch, invoker.clone(), auto_id, effective_ff))
        .collect();

    let cx = crate::dsl::custom::CustomElementContext {
        tag: custom.tag.clone(),
        props: custom.props,
        children: children_elements,
    };

    if let Some(renderer) = crate::dsl::custom::get_custom_element(&custom.tag) {
        renderer(cx)
    } else {
        log::warn!("Custom element '{}' not registered", custom.tag);
        div()
            .child(format!("<unknown custom element '{}'>", custom.tag))
            .into_any_element()
    }
}

fn apply_styles<E: Styled>(mut el: E, style: &StyleProps) -> E {
    // 1. Positioning & Layout Strategy
    if let Some(pos) = style.position {
        match pos {
            gpui::Position::Relative => el = el.relative(),
            gpui::Position::Absolute => el = el.absolute(),
        }
    }
    if let Some(t) = style.inset.top {
        match t {
            Length::Px(p) => el = el.top(px(p)),
            Length::Percent(pct) => el = el.top(relative(pct / 100.0)),
            Length::Full => el = el.top_full(),
            Length::Auto => el = el.top_auto(),
        }
    }
    if let Some(b) = style.inset.bottom {
        match b {
            Length::Px(p) => el = el.bottom(px(p)),
            Length::Percent(pct) => el = el.bottom(relative(pct / 100.0)),
            Length::Full => el = el.bottom_full(),
            Length::Auto => el = el.bottom_auto(),
        }
    }
    if let Some(l) = style.inset.left {
        match l {
            Length::Px(p) => el = el.left(px(p)),
            Length::Percent(pct) => el = el.left(relative(pct / 100.0)),
            Length::Full => el = el.left_full(),
            Length::Auto => el = el.left_auto(),
        }
    }
    if let Some(r) = style.inset.right {
        match r {
            Length::Px(p) => el = el.right(px(p)),
            Length::Percent(pct) => el = el.right(relative(pct / 100.0)),
            Length::Full => el = el.right_full(),
            Length::Auto => el = el.right_auto(),
        }
    }
    if let Some(vis) = style.visibility {
        match vis {
            gpui::Visibility::Visible => el = el.visible(),
            gpui::Visibility::Hidden => el = el.invisible(),
        }
    }

    // 2. Sizing & Aspect Ratio
    if style.aspect_square {
        el = el.aspect_square();
    } else if let Some(ratio) = style.aspect_ratio {
        el = el.aspect_ratio(ratio);
    }
    if let Some(w) = style.width {
        el = match w {
            Length::Px(p) => el.w(px(p)),
            Length::Percent(pct) => el.w(relative(pct / 100.0)),
            Length::Full => el.w_full(),
            Length::Auto => el.w_auto(),
        };
    }
    if let Some(h) = style.height {
        el = match h {
            Length::Px(p) => el.h(px(p)),
            Length::Percent(pct) => el.h(relative(pct / 100.0)),
            Length::Full => el.h_full(),
            Length::Auto => el.h_auto(),
        };
    }
    if let Some(min_w) = style.min_w {
        el = match min_w {
            Length::Px(p) => el.min_w(px(p)),
            Length::Percent(pct) => el.min_w(relative(pct / 100.0)),
            Length::Full => el.min_w_full(),
            Length::Auto => el.min_w_auto(),
        };
    }
    if let Some(min_h) = style.min_h {
        el = match min_h {
            Length::Px(p) => el.min_h(px(p)),
            Length::Percent(pct) => el.min_h(relative(pct / 100.0)),
            Length::Full => el.min_h_full(),
            Length::Auto => el.min_h_auto(),
        };
    }
    if let Some(max_w) = style.max_w {
        el = match max_w {
            Length::Px(p) => el.max_w(px(p)),
            Length::Percent(pct) => el.max_w(relative(pct / 100.0)),
            Length::Full => el.max_w_full(),
            Length::Auto => el,
        };
    }
    if let Some(max_h) = style.max_h {
        el = match max_h {
            Length::Px(p) => el.max_h(px(p)),
            Length::Percent(pct) => el.max_h(relative(pct / 100.0)),
            Length::Full => el.max_h_full(),
            Length::Auto => el,
        };
    }

    // 2. Spacing - Padding & Margin
    if let Some(l) = style.padding.top.and_then(length_to_definite) {
        el = el.pt(l);
    }
    if let Some(l) = style.padding.right.and_then(length_to_definite) {
        el = el.pr(l);
    }
    if let Some(l) = style.padding.bottom.and_then(length_to_definite) {
        el = el.pb(l);
    }
    if let Some(l) = style.padding.left.and_then(length_to_definite) {
        el = el.pl(l);
    }
    if let Some(l) = style.margin.top.and_then(length_to_definite) {
        el = el.mt(l);
    }
    if let Some(l) = style.margin.right.and_then(length_to_definite) {
        el = el.mr(l);
    }
    if let Some(l) = style.margin.bottom.and_then(length_to_definite) {
        el = el.mb(l);
    }
    if let Some(l) = style.margin.left.and_then(length_to_definite) {
        el = el.ml(l);
    }

    // 3. Visuals, Borders & Filters
    if let Some(bg) = &style.background {
        el = el.bg(bg.to_hsla());
    }
    if let Some(tc) = &style.text_color {
        el = el.text_color(tc.to_hsla());
    }
    if let Some(tbg) = &style.text_bg {
        el = el.text_bg(tbg.to_hsla());
    }
    if let Some(r) = style.corner_radius {
        el = el.rounded(px(r));
    }
    if let Some(r) = style.rounded_t {
        el = el.rounded_t(px(r));
    }
    if let Some(r) = style.rounded_b {
        el = el.rounded_b(px(r));
    }
    if let Some(r) = style.rounded_l {
        el = el.rounded_l(px(r));
    }
    if let Some(r) = style.rounded_r {
        el = el.rounded_r(px(r));
    }
    if let Some(r) = style.rounded_tl {
        el = el.rounded_tl(px(r));
    }
    if let Some(r) = style.rounded_tr {
        el = el.rounded_tr(px(r));
    }
    if let Some(r) = style.rounded_bl {
        el = el.rounded_bl(px(r));
    }
    if let Some(r) = style.rounded_br {
        el = el.rounded_br(px(r));
    }
    if style.rounded_smoothing_ios {
        el = el.rounded_smoothing_ios();
    } else if let Some(amount) = style.corner_smoothing {
        el = el.rounded_smoothing(amount);
    }
    if let Some(bw) = style.border_width {
        el = el.border(px(bw));
    }
    if let Some(bt) = style.border_widths.top {
        el = el.border_t(px(bt));
    }
    if let Some(bb) = style.border_widths.bottom {
        el = el.border_b(px(bb));
    }
    if let Some(bl) = style.border_widths.left {
        el = el.border_l(px(bl));
    }
    if let Some(br) = style.border_widths.right {
        el = el.border_r(px(br));
    }
    if let Some(bc) = &style.border_color {
        el = el.border_color(bc.to_hsla());
    }
    if style.border_dashed {
        el = el.border_dashed();
    }
    if let Some(l) = style.border_dashed_length {
        el = el.border_dashed_length(l);
    }
    if let Some(g) = style.border_dashed_gap {
        el = el.border_dashed_gap(g);
    }
    if let Some(sh) = style.shadow {
        let shadow = match sh {
            ShadowLevel::Sm => BoxShadow::new(px(0.), px(1.), black().opacity(0.1)).blur_radius(px(2.)),
            ShadowLevel::Md => BoxShadow::new(px(0.), px(4.), black().opacity(0.15)).blur_radius(px(6.)),
            ShadowLevel::Lg => BoxShadow::new(px(0.), px(10.), black().opacity(0.2)).blur_radius(px(15.)),
        };
        el = el.shadow(vec![shadow]);
    }
    if let Some(op) = style.opacity {
        el = el.opacity(op);
    }
    if let Some(b) = style.blur {
        el = el.blur(px(b));
    }
    if let Some(bb) = style.backdrop_blur {
        el = el.backdrop_blur(px(bb));
    }

    // 4. Overflow & Scrolling
    if style.overflow_hidden {
        el.style().overflow.x = Some(gpui::Overflow::Hidden);
        el.style().overflow.y = Some(gpui::Overflow::Hidden);
    }
    if style.overflow_scroll {
        el.style().overflow.x = Some(gpui::Overflow::Scroll);
        el.style().overflow.y = Some(gpui::Overflow::Scroll);
    }
    if let Some(t) = style.overflow_fade.top {
        el = el.overflow_fade_top(px(t));
    }
    if let Some(b) = style.overflow_fade.bottom {
        el = el.overflow_fade_bottom(px(b));
    }
    if let Some(l) = style.overflow_fade.left {
        el = el.overflow_fade_left(px(l));
    }
    if let Some(r) = style.overflow_fade.right {
        el = el.overflow_fade_right(px(r));
    }
    if let Some(sw) = style.scrollbar_width {
        el = el.scrollbar_width(px(sw));
    }

    // 5. Cursors
    if style.cursor_pointer {
        el = el.cursor_pointer();
    }
    if let Some(c) = style.cursor {
        el = el.cursor(c);
    }

    el
}

fn convert_svg_node(svg_node: SvgNode, invoker: Option<&Arc<dyn LuaInvoker>>) -> AnyElement {
    let mut el = svg();
    if let Some(d) = svg_node.data {
        el = el.data(d.as_bytes());
    } else if let Some(p) = svg_node.path {
        if let Some(bytes) = invoker.and_then(|i| i.get_asset(&p)) {
            el = el.data(&bytes);
        } else if let Ok(bytes) = std::fs::read(&p) {
            el = el.data(&bytes);
        } else {
            el = el.external_path(p);
        }
    }
    // GPUI's paint_svg strictly requires style.text.color to be Some(color).
    // If no text_color was specified by the user, default to white so paint_svg runs.
    if svg_node.style.text_color.is_none() {
        el = el.text_color(gpui::white());
    }
    el = apply_styles(el, &svg_node.style);
    el.into_any_element()
}

fn convert_img_node(img_node: ImgNode) -> AnyElement {
    let mut el = img(img_node.src.as_str());
    if let Some(fit) = img_node.fit {
        el = StyledImage::object_fit(el, fit.to_gpui());
    }
    el = apply_styles(el, &img_node.style);
    el.into_any_element()
}

fn convert_canvas_node(
    canvas_node: CanvasNode,
    invoker: Option<Arc<dyn LuaInvoker>>,
) -> AnyElement {
    let paint_key = canvas_node.paint;
    let invoker = invoker.clone();
    let mut el = canvas(
        move |_bounds, _window, _cx| (),
        move |bounds, _, _window, _cx| {
            if let (Some(key), Some(invoker)) = (paint_key.as_ref(), invoker.as_ref()) {
                let _ = invoker.call_key_bounds(
                    key,
                    f32::from(bounds.origin.x),
                    f32::from(bounds.origin.y),
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height),
                );
            }
        },
    );
    el = apply_styles(el, &canvas_node.style);
    el.into_any_element()
}

#[cfg(feature = "media")]
fn convert_video_node(
    video_node: VideoNode,
    invoker: Option<Arc<dyn LuaInvoker>>,
) -> AnyElement {
    let player = invoker.as_ref().and_then(|i| i.get_video_player(&video_node.src, video_node.autoplay));
    if let Some(p) = &player {
        if video_node.autoplay && !p.is_playing() {
            p.play();
        } else if !video_node.autoplay && p.is_playing() {
            p.pause();
        }
        p.set_loop(video_node.looping);
        p.set_muted(video_node.muted);
        p.set_volume(video_node.volume);
    }

    #[cfg(target_os = "windows")]
    if let Some(p) = &player {
        if let Some(surface_source) = p.current_surface() {
            let mut el = gpui::surface(surface_source);
            if let Some(fit) = video_node.fit {
                el = el.object_fit(fit.to_gpui());
            }
            let el = apply_styles(el, &video_node.style);
            return el.into_any_element();
        }
    }

    let player_clone = player.clone();
    let custom_source = gpui::ImageSource::Custom(Arc::new(move |window, _cx| {
        if let Some(p) = &player_clone {
            let current = p.current_render_image()?;
            if let Some(stale) = p.take_stale_render_image(&current) {
                let _ = window.drop_image(stale);
            }
            Some(Ok(current))
        } else {
            None
        }
    }));

    let mut el = img(custom_source);
    if let Some(fit) = video_node.fit {
        el = StyledImage::object_fit(el, fit.to_gpui());
    }
    el = apply_styles(el, &video_node.style);
    el.into_any_element()
}

#[cfg(not(feature = "media"))]
fn convert_video_node(
    video_node: VideoNode,
    _invoker: Option<Arc<dyn LuaInvoker>>,
) -> AnyElement {
    let el = div();
    let el = apply_styles(el, &video_node.style);
    el.into_any_element()
}

fn convert_text_node(text: TextNode, inherited_font_family: Option<&str>) -> AnyElement {
    let mut el = div();

    if let Some(size) = text.size {
        el = el.text_size(px(size));
    }
    if let Some(color) = text.color {
        el = el.text_color(color.to_hsla());
    }
    if text.bold {
        el = el.font_weight(FontWeight::BOLD);
    }
    if text.italic {
        el = el.italic();
    }
    if text.underline {
        el = el.underline();
    }
    if text.line_through {
        el = el.line_through();
    }
    if let Some(lh) = text.line_height {
        el = el.line_height(px(lh));
    }
    let effective_ff = text.font_family.as_deref().or(inherited_font_family);
    if let Some(ff) = effective_ff {
        el = el.font_family(SharedString::from(ff.to_string()));
    }
    if let Some(ls) = text.letter_spacing {
        el = el.letter_spacing(px(ls));
    }
    if text.truncate {
        el = el.truncate();
    }
    if let Some(lines) = text.line_clamp {
        el = el.line_clamp(lines);
    }

    el.child(SharedString::from(text.content)).into_any_element()
}

fn convert_div_node(
    div_node: DivNode,
    invoker: Option<Arc<dyn LuaInvoker>>,
    auto_id: &mut u64,
    inherited_font_family: Option<&str>,
) -> AnyElement {
    let mut el = if let Some(id_str) = div_node.id {
        div().id(SharedString::from(id_str))
    } else {
        *auto_id += 1;
        div().id(ElementId::NamedInteger(
            SharedString::new_static("lua_node"),
            *auto_id,
        ))
    };

    // 1. Display & Flexbox Layout
    if let Some(d) = div_node.style.display {
        el.style().display = Some(d);
    }
    if let Some(flex) = div_node.style.flex {
        el.style().display = Some(Display::Flex);
        el.style().flex_direction = Some(flex);
    }
    if div_node.style.flex_wrap {
        el = el.flex_wrap();
    }
    if let Some(grow) = div_node.style.flex_grow {
        el = el.flex_grow(grow);
    }
    if div_node.style.flex_1 {
        el = el.flex_1();
    }
    if div_node.style.flex_auto {
        el = el.flex_auto();
    }
    if div_node.style.flex_none {
        el = el.flex_none();
    }
    if let Some(shrink) = div_node.style.flex_shrink {
        el = el.flex_shrink(shrink);
    }
    if let Some(items) = div_node.style.items {
        el.style().align_items = Some(items);
    }
    if let Some(align_self) = div_node.style.align_self {
        el.style().align_self = Some(align_self);
    }
    if let Some(align_content) = div_node.style.align_content {
        el.style().align_content = Some(align_content);
    }
    if let Some(justify) = div_node.style.justify {
        el.style().justify_content = Some(justify);
    }
    if let Some(gap) = div_node.style.gap {
        el = el.gap(px(gap));
    }
    if let Some(gap_x) = div_node.style.gap_x {
        el = el.gap_x(px(gap_x));
    }
    if let Some(gap_y) = div_node.style.gap_y {
        el = el.gap_y(px(gap_y));
    }

    // 2. CSS Grid
    if div_node.style.grid {
        el = el.grid();
    }
    if let Some(cols) = div_node.style.grid_cols {
        el = el.grid_cols(cols);
    }
    if let Some(rows) = div_node.style.grid_rows {
        el = el.grid_rows(rows);
    }
    if div_node.style.col_span_full {
        el = el.col_span_full();
    } else if let Some(span) = div_node.style.col_span {
        el = el.col_span(span);
    }
    if div_node.style.row_span_full {
        el = el.row_span_full();
    } else if let Some(span) = div_node.style.row_span {
        el = el.row_span(span);
    }
    if let Some(start) = div_node.style.col_start {
        el = el.col_start(start);
    }
    if let Some(end) = div_node.style.col_end {
        el = el.col_end(end);
    }
    if let Some(start) = div_node.style.row_start {
        el = el.row_start(start);
    }
    if let Some(end) = div_node.style.row_end {
        el = el.row_end(end);
    }

    // 3. Shared Styling (Sizing, Spacing, Visuals, Borders, Filters, Overflow, Cursors)
    el = apply_styles(el, &div_node.style);

    // 4. Typography (Cascading)
    if div_node.style.text_left {
        el = el.text_left();
    } else if div_node.style.text_center {
        el = el.text_center();
    } else if div_node.style.text_right {
        el = el.text_right();
    }
    if div_node.style.truncate {
        el = el.truncate();
    }
    if let Some(lines) = div_node.style.line_clamp {
        el = el.line_clamp(lines);
    }
    if let Some(ls) = div_node.style.letter_spacing {
        el = el.letter_spacing(px(ls));
    }
    if div_node.style.underline {
        el = el.underline();
    }
    if div_node.style.line_through {
        el = el.line_through();
    }
    let effective_ff = div_node.style.font_family.as_deref().or(inherited_font_family);
    if let Some(ff) = effective_ff {
        el = el.font_family(SharedString::from(ff.to_string()));
    }

    // 5. Transitions
    if let Some(trans) = div_node.transitions {
        el = el.transitions(|mut t| {
            if let Some(ms) = trans.all.or(trans.opacity) {
                t = t.opacity(Duration::from_millis(ms));
            }
            if let Some(ms) = trans.all.or(trans.background) {
                t = t.bg(Duration::from_millis(ms));
            }
            if let Some(ms) = trans.all.or(trans.width) {
                t = t.w(Duration::from_millis(ms));
            }
            if let Some(ms) = trans.all.or(trans.height) {
                t = t.h(Duration::from_millis(ms));
            }
            if let Some(ms) = trans.all.or(trans.corner_radius) {
                t = t.rounded(Duration::from_millis(ms));
            }
            if let Some(ms) = trans.all.or(trans.flex_grow) {
                t = t.flex_grow(Duration::from_millis(ms));
            }
            t
        });
    }

    // 5b. Window Control Hitbox & Stop Propagation
    if let Some(control) = div_node.window_control {
        el = el.window_control_area(control);
    }

    let is_control_button = matches!(
        div_node.window_control,
        Some(gpui::WindowControlArea::Min | gpui::WindowControlArea::Max | gpui::WindowControlArea::Close)
    );
    let stop_prop = div_node.stop_propagation || is_control_button;

    if stop_prop {
        el = el.on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation());
    }

    // 6. Event Listeners
    if let (Some(key), Some(invoker)) = (div_node.on_click, invoker.as_ref()) {
        let invoker = invoker.clone();
        let key = key.clone();
        el = el.on_click(move |_event, _window, cx| {
            if stop_prop {
                cx.stop_propagation();
            }
            if let Err(e) = invoker.call_key(&key) {
                log::error!("Lua on_click error: {e}");
            }
        });
    }

    if let (Some(key), Some(invoker)) = (div_node.on_mouse_down, invoker.as_ref()) {
        let invoker = invoker.clone();
        let key = key.clone();
        el = el.on_mouse_down(gpui::MouseButton::Left, move |e, _window, cx| {
            if stop_prop { cx.stop_propagation(); }
            let _ = invoker.call_key_mouse(&key, f32::from(e.position.x), f32::from(e.position.y), "left");
        });
    }

    if let (Some(key), Some(invoker)) = (div_node.on_mouse_up, invoker.as_ref()) {
        let invoker = invoker.clone();
        let key = key.clone();
        el = el.on_mouse_up(gpui::MouseButton::Left, move |e, _window, cx| {
            if stop_prop { cx.stop_propagation(); }
            let _ = invoker.call_key_mouse(&key, f32::from(e.position.x), f32::from(e.position.y), "left");
        });
    }

    if let (Some(key), Some(invoker)) = (div_node.on_mouse_move, invoker.as_ref()) {
        let invoker = invoker.clone();
        let key = key.clone();
        el = el.on_mouse_move(move |e, _window, cx| {
            if stop_prop { cx.stop_propagation(); }
            let _ = invoker.call_key_mouse(&key, f32::from(e.position.x), f32::from(e.position.y), "move");
        });
    }

    if div_node.focusable || div_node.on_key_down.is_some() || div_node.tab_index.is_some() {
        el = el.focusable();
    }
    if let Some(tab_idx) = div_node.tab_index {
        el = el.tab_index(tab_idx);
    }

    if let (Some(key), Some(invoker)) = (div_node.on_key_down, invoker.as_ref()) {
        let invoker = invoker.clone();
        let key = key.clone();
        el = el.on_key_down(move |e, _window, cx| {
            if stop_prop { cx.stop_propagation(); }
            let m = &e.keystroke.modifiers;
            let _ = invoker.call_key_key(&key, &e.keystroke.key, e.keystroke.key_char.as_deref(), m.control, m.alt, m.shift, m.platform);
        });
    }

    if let (Some(key), Some(invoker)) = (div_node.on_key_up, invoker.as_ref()) {
        let invoker = invoker.clone();
        let key = key.clone();
        el = el.on_key_up(move |e, _window, cx| {
            if stop_prop { cx.stop_propagation(); }
            let m = &e.keystroke.modifiers;
            let _ = invoker.call_key_key(&key, &e.keystroke.key, e.keystroke.key_char.as_deref(), m.control, m.alt, m.shift, m.platform);
        });
    }

    if let (Some(key), Some(invoker)) = (div_node.on_scroll_wheel, invoker.as_ref()) {
        let invoker = invoker.clone();
        let key = key.clone();
        el = el.on_scroll_wheel(move |e, _window, cx| {
            if stop_prop { cx.stop_propagation(); }
            let (dx, dy) = match e.delta {
                gpui::ScrollDelta::Pixels(p) => (f32::from(p.x), f32::from(p.y)),
                gpui::ScrollDelta::Lines(l) => (l.x * 20.0, l.y * 20.0),
            };
            let _ = invoker.call_key_scroll(&key, dx, dy);
        });
    }

    if let (Some(key), Some(invoker)) = (div_node.on_hover, invoker.as_ref()) {
        let invoker = invoker.clone();
        let key = key.clone();
        el = el.on_hover(move |hovered, _window, _cx| {
            if let Err(e) = invoker.call_key_bool(&key, *hovered) {
                log::error!("Lua on_hover error: {e}");
            }
        });
    }

    if let (Some(key), Some(invoker)) = (div_node.on_drop, invoker.as_ref()) {
        let invoker = invoker.clone();
        let key = key.clone();
        el = el.on_drop(move |paths: &gpui::ExternalPaths, _window, _cx| {
            let path_strs: Vec<String> = paths
                .paths()
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            if let Err(e) = invoker.call_key_paths(&key, path_strs) {
                log::error!("Lua on_drop error: {e}");
            }
        });
    }

    // 7. Children
    for child in div_node.children {
        el = el.child(convert_node_recursive(child, invoker.clone(), auto_id, effective_ff));
    }

    // 8. Animations
    if let Some(anim) = div_node.animation {
        let mut animation = Animation::new(Duration::from_millis(anim.duration_ms));
        if anim.repeat {
            animation = animation.repeat();
        }
        match anim.easing.as_deref() {
            Some("linear") => animation = animation.with_easing(|d| d),
            Some("ease_in") => animation = animation.with_easing(|d| d * d),
            Some("ease_out") => animation = animation.with_easing(|d| 1.0 - (1.0 - d) * (1.0 - d)),
            Some("ease_in_out") => animation = animation.with_easing(gpui::ease_in_out),
            _ => {}
        }

        let anim_id = ElementId::NamedInteger(SharedString::new_static("lua_anim"), *auto_id);
        el.with_animation(anim_id, animation, |element, delta| {
            element.opacity(delta)
        })
        .into_any_element()
    } else {
        el.into_any_element()
    }
}
fn convert_input_node(
    mut input_node: InputNode,
    invoker: Option<Arc<dyn LuaInvoker>>,
    inherited_font_family: Option<&str>,
) -> AnyElement {
    if input_node.style.font_family.is_none() {
        if let Some(ff) = inherited_font_family {
            input_node.style.font_family = Some(ff.to_string());
        }
    }
    if input_node.multiline {
        crate::input::TextareaElement::new(input_node, invoker).into_any_element()
    } else {
        crate::input::InputElement::new(input_node, invoker).into_any_element()
    }
}
