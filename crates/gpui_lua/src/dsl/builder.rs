use crate::dsl::node::{
    AnimationProps, CanvasNode, ColorSpec, DivNode, Edges, ImageFit, ImgNode, InputNode, Length,
    LuaNode, ShadowLevel, StyleProps, SvgNode, SyncRegistryKey, TextNode, TransitionsProps, VideoNode,
};
use gpui::{
    AlignContent, AlignItems, AlignSelf, CursorStyle, Display, FlexDirection, JustifyContent,
};
use mlua::{Function, Lua, Result, Table, UserData, UserDataMethods, Value};
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum BuilderKind {
    Div(DivNode),
    Text(TextNode),
    Svg(SvgNode),
    Img(ImgNode),
    Canvas(CanvasNode),
    Video(VideoNode),
    Input(InputNode),
    Custom(crate::dsl::node::CustomNode),
}

#[derive(Clone, Debug)]
pub struct LuaElementBuilder {
    pub inner: Arc<Mutex<BuilderKind>>,
}

impl LuaElementBuilder {
    pub fn new_div() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BuilderKind::Div(DivNode::default()))),
        }
    }

    pub fn new_text(content: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BuilderKind::Text(TextNode {
                content,
                ..Default::default()
            }))),
        }
    }

    pub fn new_svg(path: Option<String>, data: Option<String>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BuilderKind::Svg(SvgNode {
                path,
                data,
                ..Default::default()
            }))),
        }
    }

    pub fn new_img(src: String, fit: Option<ImageFit>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BuilderKind::Img(ImgNode {
                src,
                fit,
                ..Default::default()
            }))),
        }
    }

    pub fn new_canvas(paint: Option<SyncRegistryKey>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BuilderKind::Canvas(CanvasNode {
                paint,
                ..Default::default()
            }))),
        }
    }

    pub fn new_video(src: String, fit: Option<ImageFit>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BuilderKind::Video(VideoNode {
                src,
                fit,
                ..Default::default()
            }))),
        }
    }

    pub fn new_input(input: InputNode) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BuilderKind::Input(input))),
        }
    }

    pub fn new_custom(tag: String, props: serde_json::Value) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BuilderKind::Custom(crate::dsl::node::CustomNode {
                tag,
                props,
                children: Vec::new(),
                style: StyleProps::default(),
            }))),
        }
    }

    pub fn with_custom_mut<F: FnOnce(&mut crate::dsl::node::CustomNode)>(&self, f: F) {
        if let BuilderKind::Custom(c) = &mut *self.inner.lock() {
            f(c);
        }
    }

    pub fn to_node(&self) -> LuaNode {
        match &*self.inner.lock() {
            BuilderKind::Div(div) => LuaNode::Div(div.clone()),
            BuilderKind::Text(text) => LuaNode::Text(text.clone()),
            BuilderKind::Svg(svg) => LuaNode::Svg(svg.clone()),
            BuilderKind::Img(img) => LuaNode::Img(img.clone()),
            BuilderKind::Canvas(canvas) => LuaNode::Canvas(canvas.clone()),
            BuilderKind::Video(video) => LuaNode::Video(video.clone()),
            BuilderKind::Input(input) => LuaNode::Input(input.clone()),
            BuilderKind::Custom(custom) => LuaNode::Custom(custom.clone()),
        }
    }
    pub fn with_style_mut<F: FnOnce(&mut StyleProps)>(&self, f: F) {
        match &mut *self.inner.lock() {
            BuilderKind::Div(d) => f(&mut d.style),
            BuilderKind::Svg(s) => f(&mut s.style),
            BuilderKind::Img(i) => f(&mut i.style),
            BuilderKind::Canvas(c) => f(&mut c.style),
            BuilderKind::Video(v) => f(&mut v.style),
            BuilderKind::Input(i) => f(&mut i.style),
            BuilderKind::Custom(c) => f(&mut c.style),
            BuilderKind::Text(_) => {}
        }
    }

    pub fn with_div_mut<F: FnOnce(&mut DivNode)>(&self, f: F) {
        if let BuilderKind::Div(div) = &mut *self.inner.lock() {
            f(div);
        }
    }

    pub fn with_text_mut<F: FnOnce(&mut TextNode)>(&self, f: F) {
        if let BuilderKind::Text(text) = &mut *self.inner.lock() {
            f(text);
        }
    }

    pub fn with_input_mut<F: FnOnce(&mut InputNode)>(&self, f: F) {
        if let BuilderKind::Input(input) = &mut *self.inner.lock() {
            f(input);
        }
    }
}

impl mlua::FromLua for LuaElementBuilder {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "LuaElementBuilder".to_string(),
                message: None,
            }),
        }
    }
}

pub fn parse_color_value(val: Value) -> Option<ColorSpec> {
    match val {
        Value::String(s) => Some(ColorSpec::Hex(s.to_str().ok()?.to_string())),
        Value::Integer(n) => Some(ColorSpec::Hex(format!("#{:06x}", n & 0xffffff))),
        Value::Table(t) => {
            if let (Ok(r), Ok(g), Ok(b)) = (
                t.get::<u8>("r").or_else(|_| t.get::<u8>(1)),
                t.get::<u8>("g").or_else(|_| t.get::<u8>(2)),
                t.get::<u8>("b").or_else(|_| t.get::<u8>(3)),
            ) {
                if let Ok(a) = t.get::<f32>("a").or_else(|_| t.get::<f32>(4)) {
                    Some(ColorSpec::Rgba(r, g, b, a))
                } else {
                    Some(ColorSpec::Rgb(r, g, b))
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn parse_cursor_style(s: &str) -> Option<CursorStyle> {
    match s.trim().to_lowercase().as_str() {
        "pointer" => Some(CursorStyle::PointingHand),
        "default" | "arrow" => Some(CursorStyle::Arrow),
        "text" | "ibeam" => Some(CursorStyle::IBeam),
        "move" | "closed_hand" => Some(CursorStyle::ClosedHand),
        "grab" | "open_hand" => Some(CursorStyle::OpenHand),
        "not_allowed" => Some(CursorStyle::OperationNotAllowed),
        "crosshair" => Some(CursorStyle::Crosshair),
        "context_menu" => Some(CursorStyle::ContextualMenu),
        _ => None,
    }
}

pub fn extract_media_source_string(val: Value) -> Option<String> {
    match val {
        Value::String(s) => s.to_str().ok().map(|s| s.to_string()),
        Value::UserData(ud) => {
            #[cfg(feature = "media")]
            if let Ok(cam) = ud.borrow::<crate::stdlib::media::LuaCameraCapture>() {
                return Some(cam.inner.src().to_string());
            } else if let Ok(mic) = ud.borrow::<crate::stdlib::media::LuaMicrophoneCapture>() {
                return Some(mic.inner.src().to_string());
            }
            #[cfg(feature = "webrtc")]
            if let Ok(t) = ud.borrow::<crate::stdlib::webrtc::LuaRemoteTrack>() {
                return Some(t.uri.clone());
            } else if let Ok(t) = ud.borrow::<crate::stdlib::webrtc::LuaLocalTrack>() {
                return Some(t.uri.clone());
            }
            None
        }
        Value::Table(tbl) => {
            if let Ok(method) = tbl.get::<Function>("src") {
                method.call::<String>(()).ok()
            } else if let Ok(s) = tbl.get::<String>("src") {
                Some(s)
            } else if let Ok(s) = tbl.get::<String>("uri") {
                Some(s)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn parse_video_table(t: Table) -> Result<LuaElementBuilder> {
    let src = if let Ok(s) = t.get::<String>("src").or_else(|_| t.get::<String>(1)) {
        s
    } else if let Ok(val) = t.get::<Value>("src")
        .or_else(|_| t.get::<Value>("track"))
        .or_else(|_| t.get::<Value>("camera"))
        .or_else(|_| t.get::<Value>("stream"))
        .or_else(|_| t.get::<Value>(1))
    {
        extract_media_source_string(val).unwrap_or_default()
    } else {
        String::new()
    };

    let fit = t
        .get::<String>("fit")
        .ok()
        .and_then(|f| match f.to_lowercase().as_str() {
            "contain" => Some(ImageFit::Contain),
            "cover" => Some(ImageFit::Cover),
            "fill" => Some(ImageFit::Fill),
            "scale_down" => Some(ImageFit::ScaleDown),
            _ => None,
        });

    let autoplay = t.get::<bool>("autoplay").unwrap_or(true);
    let looping = t.get::<bool>("loop").unwrap_or(false);
    let muted = t.get::<bool>("muted").unwrap_or(false);
    let volume = t.get::<f32>("volume").unwrap_or(1.0);

    let mut video = VideoNode {
        id: t.get::<String>("id").ok(),
        src,
        fit,
        autoplay,
        looping,
        muted,
        volume,
        style: StyleProps::default(),
    };

    parse_style_props_table(&t, &mut video.style);

    let builder = LuaElementBuilder::new_div();
    *builder.inner.lock() = BuilderKind::Video(video);
    Ok(builder)
}

pub fn parse_length_value(val: Value) -> Option<Length> {
    match val {
        Value::Integer(n) => Some(Length::Px(n as f32)),
        Value::Number(n) => Some(Length::Px(n as f32)),
        Value::String(s) => {
            let str_val = s.to_str().ok()?;
            let lower = str_val.trim().to_lowercase();
            if lower == "full" {
                Some(Length::Full)
            } else if lower == "auto" {
                Some(Length::Auto)
            } else if lower.ends_with('%') {
                let num_part = lower.trim_end_matches('%').trim();
                let pct: f32 = num_part.parse().ok()?;
                Some(Length::Percent(pct))
            } else if let Ok(num) = lower.parse::<f32>() {
                Some(Length::Px(num))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn parse_style_props_table(t: &Table, style: &mut StyleProps) {
    // Positioning & Layout Strategy
    if t.get::<bool>("relative").unwrap_or(false) {
        style.position = Some(gpui::Position::Relative);
    } else if t.get::<bool>("absolute").unwrap_or(false) {
        style.position = Some(gpui::Position::Absolute);
    } else if let Ok(pos_str) = t.get::<String>("position") {
        match pos_str.to_lowercase().as_str() {
            "relative" => style.position = Some(gpui::Position::Relative),
            "absolute" => style.position = Some(gpui::Position::Absolute),
            _ => {}
        }
    }

    // Insets (top, bottom, left, right, inset, inset_0, inset_x, inset_y)
    if t.get::<bool>("inset_0").unwrap_or(false) {
        style.inset = Edges::all(Length::Px(0.0));
    }
    if let Ok(ins) = t.get::<Value>("inset") {
        if let Some(l) = parse_length_value(ins) {
            style.inset = Edges::all(l);
        }
    }
    if let Ok(ins_x) = t.get::<Value>("inset_x") {
        if let Some(l) = parse_length_value(ins_x) {
            style.inset.left = Some(l);
            style.inset.right = Some(l);
        }
    }
    if let Ok(ins_y) = t.get::<Value>("inset_y") {
        if let Some(l) = parse_length_value(ins_y) {
            style.inset.top = Some(l);
            style.inset.bottom = Some(l);
        }
    }
    if let Ok(top_val) = t.get::<Value>("top") {
        if let Some(l) = parse_length_value(top_val) {
            style.inset.top = Some(l);
        }
    }
    if let Ok(bottom_val) = t.get::<Value>("bottom") {
        if let Some(l) = parse_length_value(bottom_val) {
            style.inset.bottom = Some(l);
        }
    }
    if let Ok(left_val) = t.get::<Value>("left") {
        if let Some(l) = parse_length_value(left_val) {
            style.inset.left = Some(l);
        }
    }
    if let Ok(right_val) = t.get::<Value>("right") {
        if let Some(l) = parse_length_value(right_val) {
            style.inset.right = Some(l);
        }
    }

    // Visibility
    if t.get::<bool>("invisible").unwrap_or(false) {
        style.visibility = Some(gpui::Visibility::Hidden);
    } else if t.get::<bool>("visible").unwrap_or(false) {
        style.visibility = Some(gpui::Visibility::Visible);
    } else if let Ok(vis_str) = t.get::<String>("visibility") {
        match vis_str.to_lowercase().as_str() {
            "hidden" | "invisible" => style.visibility = Some(gpui::Visibility::Hidden),
            "visible" => style.visibility = Some(gpui::Visibility::Visible),
            _ => {}
        }
    }

    // Sizing & Aspect Ratio
    if let Ok(r) = t.get::<f32>("aspect_ratio") {
        style.aspect_ratio = Some(r);
    }
    if t.get::<bool>("aspect_square").unwrap_or(false) {
        style.aspect_square = true;
    }
    if let Ok(w) = t.get::<Value>("w") {
        if let Some(l) = parse_length_value(w) {
            style.width = Some(l);
        }
    }
    if t.get::<bool>("w_full").unwrap_or(false) {
        style.width = Some(Length::Full);
    }
    if let Ok(h) = t.get::<Value>("h") {
        if let Some(l) = parse_length_value(h) {
            style.height = Some(l);
        }
    }
    if t.get::<bool>("h_full").unwrap_or(false) {
        style.height = Some(Length::Full);
    }
    if let Ok(min_w) = t.get::<Value>("min_w") {
        if let Some(l) = parse_length_value(min_w) {
            style.min_w = Some(l);
        }
    }
    if let Ok(min_h) = t.get::<Value>("min_h") {
        if let Some(l) = parse_length_value(min_h) {
            style.min_h = Some(l);
        }
    }
    if let Ok(max_w) = t.get::<Value>("max_w") {
        if let Some(l) = parse_length_value(max_w) {
            style.max_w = Some(l);
        }
    }
    if let Ok(max_h) = t.get::<Value>("max_h") {
        if let Some(l) = parse_length_value(max_h) {
            style.max_h = Some(l);
        }
    }

    // Padding
    if let Ok(p) = t.get::<Value>("p") {
        if let Some(len) = parse_length_value(p) {
            style.padding = Edges::all(len);
        }
    }
    if let Ok(px) = t.get::<Value>("px") {
        if let Some(len) = parse_length_value(px) {
            style.padding.left = Some(len);
            style.padding.right = Some(len);
        }
    }
    if let Ok(py) = t.get::<Value>("py") {
        if let Some(len) = parse_length_value(py) {
            style.padding.top = Some(len);
            style.padding.bottom = Some(len);
        }
    }
    if let Ok(pt) = t.get::<Value>("pt") {
        if let Some(l) = parse_length_value(pt) {
            style.padding.top = Some(l);
        }
    }
    if let Ok(pr) = t.get::<Value>("pr") {
        if let Some(l) = parse_length_value(pr) {
            style.padding.right = Some(l);
        }
    }
    if let Ok(pb) = t.get::<Value>("pb") {
        if let Some(l) = parse_length_value(pb) {
            style.padding.bottom = Some(l);
        }
    }
    if let Ok(pl) = t.get::<Value>("pl") {
        if let Some(l) = parse_length_value(pl) {
            style.padding.left = Some(l);
        }
    }

    // Margin
    if let Ok(m) = t.get::<Value>("m") {
        if let Some(len) = parse_length_value(m) {
            style.margin = Edges::all(len);
        }
    }
    if let Ok(mx) = t.get::<Value>("mx") {
        if let Some(len) = parse_length_value(mx) {
            style.margin.left = Some(len);
            style.margin.right = Some(len);
        }
    }
    if let Ok(my) = t.get::<Value>("my") {
        if let Some(len) = parse_length_value(my) {
            style.margin.top = Some(len);
            style.margin.bottom = Some(len);
        }
    }
    if let Ok(mt) = t.get::<Value>("mt") {
        if let Some(l) = parse_length_value(mt) {
            style.margin.top = Some(l);
        }
    }
    if let Ok(mr) = t.get::<Value>("mr") {
        if let Some(l) = parse_length_value(mr) {
            style.margin.right = Some(l);
        }
    }
    if let Ok(mb) = t.get::<Value>("mb") {
        if let Some(l) = parse_length_value(mb) {
            style.margin.bottom = Some(l);
        }
    }
    if let Ok(ml) = t.get::<Value>("ml") {
        if let Some(l) = parse_length_value(ml) {
            style.margin.left = Some(l);
        }
    }

    // Visuals & Filters
    if let Ok(bg) = t.get::<Value>("bg") {
        if let Some(c) = parse_color_value(bg) {
            style.background = Some(c);
        }
    }
    if let Ok(tc) = t.get::<Value>("text_color") {
        if let Some(c) = parse_color_value(tc) {
            style.text_color = Some(c);
        }
    }
    if let Ok(tbg) = t.get::<Value>("text_bg") {
        if let Some(c) = parse_color_value(tbg) {
            style.text_bg = Some(c);
        }
    }
    if let Ok(r) = t.get::<f32>("rounded") {
        style.corner_radius = Some(r);
    }
    if let Ok(amount) = t.get::<f32>("corner_smoothing") {
        style.corner_smoothing = Some(amount);
    }
    if t.get::<bool>("rounded_smoothing_ios").unwrap_or(false) {
        style.rounded_smoothing_ios = true;
    }
    // Per-edge borders
    if let Ok(bt) = t.get::<f32>("border_t") {
        style.border_widths.top = Some(bt);
    }
    if let Ok(bb) = t.get::<f32>("border_b") {
        style.border_widths.bottom = Some(bb);
    }
    if let Ok(bl) = t.get::<f32>("border_l") {
        style.border_widths.left = Some(bl);
    }
    if let Ok(br) = t.get::<f32>("border_r") {
        style.border_widths.right = Some(br);
    }
    if let Ok(bx) = t.get::<f32>("border_x") {
        style.border_widths.left = Some(bx);
        style.border_widths.right = Some(bx);
    }
    if let Ok(by) = t.get::<f32>("border_y") {
        style.border_widths.top = Some(by);
        style.border_widths.bottom = Some(by);
    }

    // Per-corner rounded
    if let Ok(r) = t.get::<f32>("rounded_t") {
        style.rounded_t = Some(r);
    }
    if let Ok(r) = t.get::<f32>("rounded_b") {
        style.rounded_b = Some(r);
    }
    if let Ok(r) = t.get::<f32>("rounded_l") {
        style.rounded_l = Some(r);
    }
    if let Ok(r) = t.get::<f32>("rounded_r") {
        style.rounded_r = Some(r);
    }
    if let Ok(r) = t.get::<f32>("rounded_tl") {
        style.rounded_tl = Some(r);
    }
    if let Ok(r) = t.get::<f32>("rounded_tr") {
        style.rounded_tr = Some(r);
    }
    if let Ok(r) = t.get::<f32>("rounded_bl") {
        style.rounded_bl = Some(r);
    }
    if let Ok(r) = t.get::<f32>("rounded_br") {
        style.rounded_br = Some(r);
    }
    if let Ok(bw) = t.get::<f32>("border") {
        style.border_width = Some(bw);
    }
    if let Ok(bc) = t.get::<Value>("border_color") {
        if let Some(c) = parse_color_value(bc) {
            style.border_color = Some(c);
        }
    }
    if t.get::<bool>("border_dashed").unwrap_or(false) {
        style.border_dashed = true;
    }
    if let Ok(l) = t.get::<f32>("border_dashed_length") {
        style.border_dashed_length = Some(l);
    }
    if let Ok(g) = t.get::<f32>("border_dashed_gap") {
        style.border_dashed_gap = Some(g);
    }
    if let Ok(s) = t.get::<String>("shadow") {
        match s.to_lowercase().as_str() {
            "sm" => style.shadow = Some(ShadowLevel::Sm),
            "md" => style.shadow = Some(ShadowLevel::Md),
            "lg" => style.shadow = Some(ShadowLevel::Lg),
            _ => {}
        }
    }
    if t.get::<bool>("shadow_sm").unwrap_or(false) {
        style.shadow = Some(ShadowLevel::Sm);
    }
    if t.get::<bool>("shadow_md").unwrap_or(false) {
        style.shadow = Some(ShadowLevel::Md);
    }
    if t.get::<bool>("shadow_lg").unwrap_or(false) {
        style.shadow = Some(ShadowLevel::Lg);
    }
    if let Ok(op) = t.get::<f32>("opacity") {
        style.opacity = Some(op);
    }
    if let Ok(b) = t.get::<f32>("blur") {
        style.blur = Some(b);
    }
    if let Ok(bb) = t.get::<f32>("backdrop_blur") {
        style.backdrop_blur = Some(bb);
    }

    // Overflow & Scrolling
    if t.get::<bool>("overflow_hidden").unwrap_or(false) {
        style.overflow_hidden = true;
    }
    if t.get::<bool>("overflow_scroll").unwrap_or(false) {
        style.overflow_scroll = true;
    }
    if let Ok(fade_val) = t.get::<Value>("overflow_fade") {
        match fade_val {
            Value::Integer(i) => style.overflow_fade = Edges::all(i as f32),
            Value::Number(n) => style.overflow_fade = Edges::all(n as f32),
            Value::Table(tbl) => {
                if let Ok(v) = tbl.get::<f32>("top").or_else(|_| tbl.get::<f32>("t")) {
                    style.overflow_fade.top = Some(v);
                }
                if let Ok(v) = tbl.get::<f32>("bottom").or_else(|_| tbl.get::<f32>("b")) {
                    style.overflow_fade.bottom = Some(v);
                }
                if let Ok(v) = tbl.get::<f32>("left").or_else(|_| tbl.get::<f32>("l")) {
                    style.overflow_fade.left = Some(v);
                }
                if let Ok(v) = tbl.get::<f32>("right").or_else(|_| tbl.get::<f32>("r")) {
                    style.overflow_fade.right = Some(v);
                }
            }
            _ => {}
        }
    }
    if let Ok(fx) = t.get::<f32>("overflow_fade_x") {
        style.overflow_fade.left = Some(fx);
        style.overflow_fade.right = Some(fx);
    }
    if let Ok(fy) = t.get::<f32>("overflow_fade_y") {
        style.overflow_fade.top = Some(fy);
        style.overflow_fade.bottom = Some(fy);
    }
    if let Ok(f) = t.get::<f32>("overflow_fade_top").or_else(|_| t.get::<f32>("overflow_fade_t")) {
        style.overflow_fade.top = Some(f);
    }
    if let Ok(f) = t.get::<f32>("overflow_fade_bottom").or_else(|_| t.get::<f32>("overflow_fade_b")) {
        style.overflow_fade.bottom = Some(f);
    }
    if let Ok(f) = t.get::<f32>("overflow_fade_left").or_else(|_| t.get::<f32>("overflow_fade_l")) {
        style.overflow_fade.left = Some(f);
    }
    if let Ok(f) = t.get::<f32>("overflow_fade_right").or_else(|_| t.get::<f32>("overflow_fade_r")) {
        style.overflow_fade.right = Some(f);
    }
    if let Ok(sw) = t.get::<f32>("scrollbar_width") {
        style.scrollbar_width = Some(sw);
    }

    // Cursors
    if t.get::<bool>("cursor_pointer").unwrap_or(false) {
        style.cursor_pointer = true;
    }
    if let Ok(cs_str) = t.get::<String>("cursor") {
        style.cursor = parse_cursor_style(&cs_str);
    }
}

pub fn value_to_node(lua: &Lua, val: Value) -> Option<LuaNode> {
    match val {
        Value::UserData(ud) => {
            if let Ok(builder) = ud.borrow::<LuaElementBuilder>() {
                Some(builder.to_node())
            } else {
                None
            }
        }
        Value::String(s) => Some(LuaNode::Text(TextNode {
            content: s.to_str().ok()?.to_string(),
            ..Default::default()
        })),
        Value::Table(t) => {
            if let Ok(builder) = parse_div_table(lua, t) {
                Some(builder.to_node())
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn parse_div_table(lua: &Lua, t: Table) -> Result<LuaElementBuilder> {
    let builder = LuaElementBuilder::new_div();
    let mut div = DivNode::default();

    if let Ok(id) = t.get::<String>("id") {
        div.id = Some(id);
    }

    // Display & Flexbox
    if let Ok(d_str) = t.get::<String>("display") {
        match d_str.to_lowercase().as_str() {
            "block" => div.style.display = Some(Display::Block),
            "flex" => div.style.display = Some(Display::Flex),
            "grid" => div.style.display = Some(Display::Grid),
            "none" | "hidden" => div.style.display = Some(Display::None),
            _ => {}
        }
    }
    if t.get::<bool>("block").unwrap_or(false) {
        div.style.display = Some(Display::Block);
    }
    if t.get::<bool>("hidden").unwrap_or(false) {
        div.style.display = Some(Display::None);
    }
    if t.get::<bool>("flex").unwrap_or(false) {
        div.style.flex = Some(FlexDirection::Row);
    }
    if t.get::<bool>("flex_col").unwrap_or(false) {
        div.style.flex = Some(FlexDirection::Column);
    }
    if t.get::<bool>("flex_row").unwrap_or(false) {
        div.style.flex = Some(FlexDirection::Row);
    }
    if t.get::<bool>("flex_wrap").unwrap_or(false) {
        div.style.flex_wrap = true;
    }
    if let Ok(g) = t.get::<f32>("flex_grow") {
        div.style.flex_grow = Some(g);
    }
    if t.get::<bool>("flex_1").unwrap_or(false) {
        div.style.flex_1 = true;
    }
    if t.get::<bool>("flex_auto").unwrap_or(false) {
        div.style.flex_auto = true;
    }
    if t.get::<bool>("flex_none").unwrap_or(false) {
        div.style.flex_none = true;
    }
    if let Ok(s) = t.get::<f32>("flex_shrink") {
        div.style.flex_shrink = Some(s);
    }

    // Alignment
    if t.get::<bool>("items_center").unwrap_or(false) {
        div.style.items = Some(AlignItems::Center);
    }
    if t.get::<bool>("items_start").unwrap_or(false) {
        div.style.items = Some(AlignItems::FlexStart);
    }
    if t.get::<bool>("items_end").unwrap_or(false) {
        div.style.items = Some(AlignItems::FlexEnd);
    }
    if t.get::<bool>("items_baseline").unwrap_or(false) {
        div.style.items = Some(AlignItems::Baseline);
    }
    if t.get::<bool>("items_stretch").unwrap_or(false) {
        div.style.items = Some(AlignItems::Stretch);
    }

    // Align Self
    if t.get::<bool>("self_start").unwrap_or(false) {
        div.style.align_self = Some(AlignSelf::Start);
    }
    if t.get::<bool>("self_center").unwrap_or(false) {
        div.style.align_self = Some(AlignSelf::Center);
    }
    if t.get::<bool>("self_end").unwrap_or(false) {
        div.style.align_self = Some(AlignSelf::End);
    }
    if t.get::<bool>("self_stretch").unwrap_or(false) {
        div.style.align_self = Some(AlignSelf::Stretch);
    }

    // Justify Content
    if t.get::<bool>("justify_center").unwrap_or(false) {
        div.style.justify = Some(JustifyContent::Center);
    }
    if t.get::<bool>("justify_between").unwrap_or(false) {
        div.style.justify = Some(JustifyContent::SpaceBetween);
    }
    if t.get::<bool>("justify_start").unwrap_or(false) {
        div.style.justify = Some(JustifyContent::FlexStart);
    }
    if t.get::<bool>("justify_end").unwrap_or(false) {
        div.style.justify = Some(JustifyContent::FlexEnd);
    }
    if t.get::<bool>("justify_around").unwrap_or(false) {
        div.style.justify = Some(JustifyContent::SpaceAround);
    }
    if t.get::<bool>("justify_evenly").unwrap_or(false) {
        div.style.justify = Some(JustifyContent::SpaceEvenly);
    }

    // Align Content
    if t.get::<bool>("content_center").unwrap_or(false) {
        div.style.align_content = Some(AlignContent::Center);
    }
    if t.get::<bool>("content_start").unwrap_or(false) {
        div.style.align_content = Some(AlignContent::FlexStart);
    }
    if t.get::<bool>("content_end").unwrap_or(false) {
        div.style.align_content = Some(AlignContent::FlexEnd);
    }
    if t.get::<bool>("content_between").unwrap_or(false) {
        div.style.align_content = Some(AlignContent::SpaceBetween);
    }
    if t.get::<bool>("content_around").unwrap_or(false) {
        div.style.align_content = Some(AlignContent::SpaceAround);
    }
    if t.get::<bool>("content_evenly").unwrap_or(false) {
        div.style.align_content = Some(AlignContent::SpaceEvenly);
    }
    if t.get::<bool>("content_stretch").unwrap_or(false) {
        div.style.align_content = Some(AlignContent::Stretch);
    }

    // Gap
    if let Ok(gap) = t.get::<f32>("gap") {
        div.style.gap = Some(gap);
    }
    if let Ok(gap_x) = t.get::<f32>("gap_x") {
        div.style.gap_x = Some(gap_x);
    }
    if let Ok(gap_y) = t.get::<f32>("gap_y") {
        div.style.gap_y = Some(gap_y);
    }

    // Shared style properties
    parse_style_props_table(&t, &mut div.style);

    // Grid
    if t.get::<bool>("grid").unwrap_or(false) {
        div.style.grid = true;
    }
    if let Ok(cols) = t.get::<u16>("grid_cols") {
        div.style.grid = true;
        div.style.grid_cols = Some(cols);
    }
    if let Ok(rows) = t.get::<u16>("grid_rows") {
        div.style.grid = true;
        div.style.grid_rows = Some(rows);
    }
    if let Ok(s) = t.get::<u16>("col_span") {
        div.style.col_span = Some(s);
    }
    if t.get::<bool>("col_span_full").unwrap_or(false) {
        div.style.col_span_full = true;
    }
    if let Ok(s) = t.get::<u16>("row_span") {
        div.style.row_span = Some(s);
    }
    if t.get::<bool>("row_span_full").unwrap_or(false) {
        div.style.row_span_full = true;
    }
    if let Ok(start) = t.get::<i16>("col_start") {
        div.style.col_start = Some(start);
    }
    if let Ok(end) = t.get::<i16>("col_end") {
        div.style.col_end = Some(end);
    }
    if let Ok(start) = t.get::<i16>("row_start") {
        div.style.row_start = Some(start);
    }
    if let Ok(end) = t.get::<i16>("row_end") {
        div.style.row_end = Some(end);
    }

    // Typography
    if t.get::<bool>("text_left").unwrap_or(false) {
        div.style.text_left = true;
    }
    if t.get::<bool>("text_center").unwrap_or(false) {
        div.style.text_center = true;
    }
    if t.get::<bool>("text_right").unwrap_or(false) {
        div.style.text_right = true;
    }
    if t.get::<bool>("truncate").unwrap_or(false) {
        div.style.truncate = true;
    }
    if let Ok(lines) = t.get::<usize>("line_clamp") {
        div.style.line_clamp = Some(lines);
    }
    if let Ok(ls) = t.get::<f32>("letter_spacing") {
        div.style.letter_spacing = Some(ls);
    }
    if t.get::<bool>("underline").unwrap_or(false) {
        div.style.underline = true;
    }
    if t.get::<bool>("line_through").unwrap_or(false) {
        div.style.line_through = true;
    }
    if let Ok(ff) = t.get::<String>("font_family") {
        div.style.font_family = Some(ff);
    }

    // Transitions
    if let Ok(ms) = t.get::<u64>("transition") {
        div.transitions = Some(TransitionsProps {
            all: Some(ms),
            ..Default::default()
        });
    } else if let Ok(trans_tbl) = t.get::<Table>("transitions") {
        let mut trans = TransitionsProps::default();
        if let Ok(ms) = trans_tbl.get::<u64>("all") { trans.all = Some(ms); }
        if let Ok(ms) = trans_tbl.get::<u64>("opacity") { trans.opacity = Some(ms); }
        if let Ok(ms) = trans_tbl.get::<u64>("bg").or_else(|_| trans_tbl.get::<u64>("background")) { trans.background = Some(ms); }
        if let Ok(ms) = trans_tbl.get::<u64>("w").or_else(|_| trans_tbl.get::<u64>("width")) { trans.width = Some(ms); }
        if let Ok(ms) = trans_tbl.get::<u64>("h").or_else(|_| trans_tbl.get::<u64>("height")) { trans.height = Some(ms); }
        if let Ok(ms) = trans_tbl.get::<u64>("rounded").or_else(|_| trans_tbl.get::<u64>("corner_radius")) { trans.corner_radius = Some(ms); }
        if let Ok(ms) = trans_tbl.get::<u64>("flex_grow") { trans.flex_grow = Some(ms); }
        div.transitions = Some(trans);
    }

    // Animation
    if let Ok(anim_tbl) = t.get::<Table>("animation") {
        let duration_ms = anim_tbl.get::<u64>("duration").unwrap_or(1000);
        let repeat = anim_tbl.get::<bool>("repeat").unwrap_or(true);
        let easing = anim_tbl.get::<String>("easing").ok();
        div.animation = Some(AnimationProps {
            duration_ms,
            repeat,
            easing,
        });
    }

    // Events
    if let Ok(f) = t.get::<Function>("on_click") {
        let key = SyncRegistryKey::new(lua.create_registry_value(f)?);
        div.on_click = Some(key);
    }
    if let Ok(f) = t.get::<Function>("on_hover") {
        let key = SyncRegistryKey::new(lua.create_registry_value(f)?);
        div.on_hover = Some(key);
    }
    if let Ok(f) = t.get::<Function>("on_drop") {
        let key = SyncRegistryKey::new(lua.create_registry_value(f)?);
        div.on_drop = Some(key);
    }
    if let Ok(f) = t.get::<Function>("on_mouse_down") {
        let key = SyncRegistryKey::new(lua.create_registry_value(f)?);
        div.on_mouse_down = Some(key);
    }
    if let Ok(f) = t.get::<Function>("on_mouse_up") {
        let key = SyncRegistryKey::new(lua.create_registry_value(f)?);
        div.on_mouse_up = Some(key);
    }
    if let Ok(f) = t.get::<Function>("on_mouse_move") {
        let key = SyncRegistryKey::new(lua.create_registry_value(f)?);
        div.on_mouse_move = Some(key);
    }
    if let Ok(f) = t.get::<Function>("on_key_down") {
        let key = SyncRegistryKey::new(lua.create_registry_value(f)?);
        div.on_key_down = Some(key);
    }
    if let Ok(f) = t.get::<Function>("on_key_up") {
        let key = SyncRegistryKey::new(lua.create_registry_value(f)?);
        div.on_key_up = Some(key);
    }
    if let Ok(f) = t.get::<Function>("on_scroll_wheel") {
        let key = SyncRegistryKey::new(lua.create_registry_value(f)?);
        div.on_scroll_wheel = Some(key);
    }
    if let Ok(val) = t.get::<String>("window_control") {
        div.window_control = match val.to_lowercase().as_str() {
            "drag" => Some(gpui::WindowControlArea::Drag),
            "close" => Some(gpui::WindowControlArea::Close),
            "max" | "maximize" => Some(gpui::WindowControlArea::Max),
            "min" | "minimize" => Some(gpui::WindowControlArea::Min),
            _ => None,
        };
    }
    if let Ok(b) = t.get::<bool>("stop_propagation") {
        div.stop_propagation = b;
    }
    if let Ok(b) = t.get::<bool>("focusable") {
        div.focusable = b;
    }
    if let Ok(idx) = t.get::<isize>("tab_index") {
        div.tab_index = Some(idx);
        div.focusable = true;
    }
    // Children table
    if let Ok(children_table) = t.get::<Table>("children") {
        for pair in children_table.sequence_values::<Value>() {
            if let Ok(v) = pair {
                if let Some(child_node) = value_to_node(lua, v) {
                    div.children.push(child_node);
                }
            }
        }
    }

    // Array elements directly in table
    for v in t.sequence_values::<Value>() {
        if let Ok(val) = v {
            if let Some(child_node) = value_to_node(lua, val) {
                div.children.push(child_node);
            }
        }
    }

    *builder.inner.lock() = BuilderKind::Div(div);
    Ok(builder)
}

pub fn parse_text_table(t: Table) -> Result<LuaElementBuilder> {
    let content = t.get::<String>("content").unwrap_or_default();
    let mut text = TextNode {
        content,
        ..Default::default()
    };

    if let Ok(sz) = t.get::<f32>("size") {
        text.size = Some(sz);
    }
    if let Ok(c) = t.get::<Value>("color") {
        text.color = parse_color_value(c);
    }
    if t.get::<bool>("bold").unwrap_or(false) {
        text.bold = true;
    }
    if t.get::<bool>("italic").unwrap_or(false) {
        text.italic = true;
    }
    if t.get::<bool>("underline").unwrap_or(false) {
        text.underline = true;
    }
    if t.get::<bool>("line_through").unwrap_or(false) {
        text.line_through = true;
    }
    if let Ok(lh) = t.get::<f32>("line_height") {
        text.line_height = Some(lh);
    }
    if let Ok(ls) = t.get::<f32>("letter_spacing") {
        text.letter_spacing = Some(ls);
    }
    if t.get::<bool>("truncate").unwrap_or(false) {
        text.truncate = true;
    }
    if let Ok(lines) = t.get::<usize>("line_clamp") {
        text.line_clamp = Some(lines);
    }
    if let Ok(ff) = t.get::<String>("font_family") {
        text.font_family = Some(ff);
    }

    let builder = LuaElementBuilder::new_text(text.content.clone());
    *builder.inner.lock() = BuilderKind::Text(text);
    Ok(builder)
}

pub fn parse_svg_table(t: Table) -> Result<LuaElementBuilder> {
    let path = t.get::<String>("path").ok().or_else(|| t.get::<String>(1).ok());
    let data = t.get::<String>("data").ok();
    let builder = LuaElementBuilder::new_svg(path, data);
    if let BuilderKind::Svg(s) = &mut *builder.inner.lock() {
        if let Ok(id) = t.get::<String>("id") {
            s.id = Some(id);
        }
        parse_style_props_table(&t, &mut s.style);
    }
    Ok(builder)
}

pub fn parse_img_table(t: Table) -> Result<LuaElementBuilder> {
    let src = t
        .get::<String>("src")
        .ok()
        .or_else(|| t.get::<String>(1).ok())
        .unwrap_or_default();
    let fit = t.get::<String>("fit").ok().and_then(|f| match f.to_lowercase().as_str() {
        "contain" => Some(ImageFit::Contain),
        "cover" => Some(ImageFit::Cover),
        "fill" => Some(ImageFit::Fill),
        "none" => Some(ImageFit::None),
        "scale_down" => Some(ImageFit::ScaleDown),
        _ => None,
    });
    let builder = LuaElementBuilder::new_img(src, fit);
    if let BuilderKind::Img(i) = &mut *builder.inner.lock() {
        if let Ok(id) = t.get::<String>("id") {
            i.id = Some(id);
        }
        parse_style_props_table(&t, &mut i.style);
    }
    Ok(builder)
}

pub fn parse_input_table(lua: &Lua, t: Table, multiline: bool) -> Result<LuaElementBuilder> {
    let mut input = InputNode::default();
    input.multiline = multiline;

    if let Ok(id) = t.get::<String>("id") {
        input.id = id;
    } else if let Ok(name) = t.get::<String>("name") {
        input.id = name;
    } else {
        static INPUT_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id_num = INPUT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        input.id = format!("input_{id_num}");
    }

    if let Ok(val) = t.get::<String>("value") {
        input.value = Some(val);
    } else if let Ok(val) = t.get::<String>("default_value") {
        input.value = Some(val);
    }

    if let Ok(ph) = t.get::<String>("placeholder") {
        input.placeholder = ph;
    }
    if let Ok(ph_c) = t.get::<Value>("placeholder_color").or_else(|_| t.get::<Value>("placeholderColor")) {
        input.placeholder_color = parse_color_value(ph_c);
    }

    let is_pwd = t.get::<bool>("password").unwrap_or(false)
        || t.get::<bool>("masked").unwrap_or(false)
        || t.get::<String>("type").map_or(false, |s| s == "password");

    if is_pwd {
        let mask_str = t.get::<String>("mask").unwrap_or_else(|_| "•".to_string());
        input.mask = mask_str.chars().next().or(Some('•'));
    }

    if let Ok(rows) = t.get::<usize>("rows") {
        input.rows = rows;
    }
    if let Ok(mr) = t.get::<usize>("max_rows") {
        input.max_rows = Some(mr);
    }
    if let Ok(max) = t.get::<usize>("max_length") {
        input.max_length = Some(max);
    }
    if let Ok(pat) = t.get::<String>("pattern") {
        input.pattern = Some(pat);
    }
    if let Ok(msg) = t.get::<String>("error_message") {
        input.error_message = Some(msg);
    }
    if let Ok(req) = t.get::<bool>("required") {
        input.required = req;
    }
    if let Ok(dis) = t.get::<bool>("disabled") {
        input.disabled = dis;
    }
    if let Ok(ro) = t.get::<bool>("read_only") {
        input.read_only = ro;
    }
    if let Ok(af) = t.get::<bool>("autofocus") {
        input.autofocus = af;
    }
    if let Ok(sz) = t.get::<f32>("font_size").or_else(|_| t.get::<f32>("fontSize")).or_else(|_| t.get::<f32>("size")) {
        input.font_size = Some(sz);
    }
    if let Ok(c) = t.get::<Value>("focus_border_color").or_else(|_| t.get::<Value>("focusBorderColor")) {
        input.focus_border_color = parse_color_value(c);
    }
    if let Ok(c) = t.get::<Value>("focus_bg").or_else(|_| t.get::<Value>("focusBg")).or_else(|_| t.get::<Value>("focus_background")) {
        input.focus_bg = parse_color_value(c);
    }
    if let Ok(c) = t.get::<Value>("error_border_color").or_else(|_| t.get::<Value>("errorBorderColor")) {
        input.error_border_color = parse_color_value(c);
    }
    if let Ok(c) = t.get::<Value>("cursor_color").or_else(|_| t.get::<Value>("cursorColor")) {
        input.cursor_color = parse_color_value(c);
    }
    if let Ok(c) = t.get::<Value>("selection_bg").or_else(|_| t.get::<Value>("selectionBg")).or_else(|_| t.get::<Value>("selection_color")).or_else(|_| t.get::<Value>("selectionColor")) {
        input.selection_bg = parse_color_value(c);
    }
    // Callbacks
    if let Ok(f) = t.get::<Function>("on_change") {
        input.on_change = Some(SyncRegistryKey::new(lua.create_registry_value(f)?));
    }
    if let Ok(f) = t.get::<Function>("on_input") {
        input.on_input = Some(SyncRegistryKey::new(lua.create_registry_value(f)?));
    }
    if let Ok(f) = t.get::<Function>("on_submit") {
        input.on_submit = Some(SyncRegistryKey::new(lua.create_registry_value(f)?));
    }
    if let Ok(f) = t.get::<Function>("on_focus") {
        input.on_focus = Some(SyncRegistryKey::new(lua.create_registry_value(f)?));
    }
    if let Ok(f) = t.get::<Function>("on_blur") {
        input.on_blur = Some(SyncRegistryKey::new(lua.create_registry_value(f)?));
    }
    if let Ok(f) = t.get::<Function>("on_validate") {
        input.on_validate = Some(SyncRegistryKey::new(lua.create_registry_value(f.clone())?));
        let val = input.value.as_deref().unwrap_or("");
        let mut is_valid = true;
        let mut err = None;
        if input.required && val.is_empty() {
            is_valid = false;
            err = input.error_message.clone().or_else(|| Some("This field is required".to_string()));
        } else if let Some(pat) = &input.pattern {
            if !val.is_empty() && !regex::Regex::new(pat).map_or(true, |r| r.is_match(val)) {
                is_valid = false;
                err = input.error_message.clone().or_else(|| Some("Invalid input format".to_string()));
            }
        }
        let _ = f.call::<()>((is_valid, err));
    }

    let temp_div = parse_div_table(lua, t.clone())?;
    let mut style = match &*temp_div.inner.lock() {
        BuilderKind::Div(d) => d.style.clone(),
        _ => StyleProps::default(),
    };

    if let Ok(c) = t.get::<Value>("color").or_else(|_| t.get::<Value>("text_color")).or_else(|_| t.get::<Value>("textColor")) {
        style.text_color = parse_color_value(c);
    }
    if let Ok(c) = t.get::<Value>("bg").or_else(|_| t.get::<Value>("background")) {
        style.background = parse_color_value(c);
    }
    if let Ok(c) = t.get::<Value>("border_color").or_else(|_| t.get::<Value>("borderColor")) {
        style.border_color = parse_color_value(c);
    }
    if let Ok(bw) = t.get::<f32>("border").or_else(|_| t.get::<f32>("border_width")).or_else(|_| t.get::<f32>("borderWidth")) {
        style.border_width = Some(bw);
    }
    if let Ok(r) = t.get::<f32>("rounded").or_else(|_| t.get::<f32>("corner_radius")).or_else(|_| t.get::<f32>("cornerRadius")) {
        style.corner_radius = Some(r);
    }

    input.style = style;

    Ok(LuaElementBuilder::new_input(input))
}
pub fn parse_canvas_table(lua: &Lua, t: Table) -> Result<LuaElementBuilder> {
    let paint_key = if let Ok(f) = t.get::<Function>("paint") {
        Some(SyncRegistryKey::new(lua.create_registry_value(f)?))
    } else {
        None
    };
    let builder = LuaElementBuilder::new_canvas(paint_key);
    if let BuilderKind::Canvas(c) = &mut *builder.inner.lock() {
        if let Ok(id) = t.get::<String>("id") {
            c.id = Some(id);
        }
        parse_style_props_table(&t, &mut c.style);
    }
    Ok(builder)
}

impl UserData for LuaElementBuilder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, id: String| {
            match &mut *this.inner.lock() {
                BuilderKind::Div(d) => d.id = Some(id),
                BuilderKind::Svg(s) => s.id = Some(id),
                BuilderKind::Img(i) => i.id = Some(id),
                BuilderKind::Canvas(c) => c.id = Some(id),
                BuilderKind::Video(v) => v.id = Some(id),
                BuilderKind::Input(inp) => inp.id = id,
                BuilderKind::Text(_) | BuilderKind::Custom(_) => {}
            }
            Ok(this.clone())
        });

        // Display
        methods.add_method("block", |_, this, ()| {
            this.with_style_mut(|s| s.display = Some(Display::Block));
            Ok(this.clone())
        });

        methods.add_method("hidden", |_, this, ()| {
            this.with_style_mut(|s| s.display = Some(Display::None));
            Ok(this.clone())
        });

        // Visibility
        methods.add_method("visible", |_, this, ()| {
            this.with_style_mut(|s| s.visibility = Some(gpui::Visibility::Visible));
            Ok(this.clone())
        });

        methods.add_method("invisible", |_, this, ()| {
            this.with_style_mut(|s| s.visibility = Some(gpui::Visibility::Hidden));
            Ok(this.clone())
        });

        // Positioning
        methods.add_method("relative", |_, this, ()| {
            this.with_style_mut(|s| s.position = Some(gpui::Position::Relative));
            Ok(this.clone())
        });

        methods.add_method("absolute", |_, this, ()| {
            this.with_style_mut(|s| s.position = Some(gpui::Position::Absolute));
            Ok(this.clone())
        });

        methods.add_method("top", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| s.inset.top = Some(l));
            }
            Ok(this.clone())
        });

        methods.add_method("bottom", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| s.inset.bottom = Some(l));
            }
            Ok(this.clone())
        });

        methods.add_method("left", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| s.inset.left = Some(l));
            }
            Ok(this.clone())
        });

        methods.add_method("right", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| s.inset.right = Some(l));
            }
            Ok(this.clone())
        });

        methods.add_method("inset", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| s.inset = Edges::all(l));
            }
            Ok(this.clone())
        });

        methods.add_method("inset_0", |_, this, ()| {
            this.with_style_mut(|s| s.inset = Edges::all(Length::Px(0.0)));
            Ok(this.clone())
        });

        methods.add_method("inset_x", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| {
                    s.inset.left = Some(l);
                    s.inset.right = Some(l);
                });
            }
            Ok(this.clone())
        });

        methods.add_method("inset_y", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| {
                    s.inset.top = Some(l);
                    s.inset.bottom = Some(l);
                });
            }
            Ok(this.clone())
        });

        // Flexbox methods
        methods.add_method("flex", |_, this, ()| {
            this.with_style_mut(|s| s.flex = Some(FlexDirection::Row));
            Ok(this.clone())
        });

        methods.add_method("flex_col", |_, this, ()| {
            this.with_style_mut(|s| s.flex = Some(FlexDirection::Column));
            Ok(this.clone())
        });

        methods.add_method("flex_row", |_, this, ()| {
            this.with_style_mut(|s| s.flex = Some(FlexDirection::Row));
            Ok(this.clone())
        });

        methods.add_method("flex_wrap", |_, this, wrap: Option<bool>| {
            this.with_style_mut(|s| s.flex_wrap = wrap.unwrap_or(true));
            Ok(this.clone())
        });

        methods.add_method("flex_grow", |_, this, grow: Option<f32>| {
            this.with_style_mut(|s| s.flex_grow = Some(grow.unwrap_or(1.0)));
            Ok(this.clone())
        });

        methods.add_method("flex_1", |_, this, ()| {
            this.with_style_mut(|s| s.flex_1 = true);
            Ok(this.clone())
        });

        methods.add_method("flex_auto", |_, this, ()| {
            this.with_style_mut(|s| s.flex_auto = true);
            Ok(this.clone())
        });

        methods.add_method("flex_none", |_, this, ()| {
            this.with_style_mut(|s| s.flex_none = true);
            Ok(this.clone())
        });

        methods.add_method("flex_shrink", |_, this, shrink: Option<f32>| {
            this.with_style_mut(|s| s.flex_shrink = Some(shrink.unwrap_or(1.0)));
            Ok(this.clone())
        });

        // Alignment
        methods.add_method("items_center", |_, this, ()| {
            this.with_style_mut(|s| s.items = Some(AlignItems::Center));
            Ok(this.clone())
        });

        methods.add_method("items_start", |_, this, ()| {
            this.with_style_mut(|s| s.items = Some(AlignItems::FlexStart));
            Ok(this.clone())
        });

        methods.add_method("items_end", |_, this, ()| {
            this.with_style_mut(|s| s.items = Some(AlignItems::FlexEnd));
            Ok(this.clone())
        });

        methods.add_method("items_baseline", |_, this, ()| {
            this.with_style_mut(|s| s.items = Some(AlignItems::Baseline));
            Ok(this.clone())
        });

        methods.add_method("items_stretch", |_, this, ()| {
            this.with_style_mut(|s| s.items = Some(AlignItems::Stretch));
            Ok(this.clone())
        });

        // Align Self
        methods.add_method("self_start", |_, this, ()| {
            this.with_style_mut(|s| s.align_self = Some(AlignSelf::Start));
            Ok(this.clone())
        });

        methods.add_method("self_center", |_, this, ()| {
            this.with_style_mut(|s| s.align_self = Some(AlignSelf::Center));
            Ok(this.clone())
        });

        methods.add_method("self_end", |_, this, ()| {
            this.with_style_mut(|s| s.align_self = Some(AlignSelf::End));
            Ok(this.clone())
        });

        methods.add_method("self_stretch", |_, this, ()| {
            this.with_style_mut(|s| s.align_self = Some(AlignSelf::Stretch));
            Ok(this.clone())
        });

        // Justify Content
        methods.add_method("justify_center", |_, this, ()| {
            this.with_style_mut(|s| s.justify = Some(JustifyContent::Center));
            Ok(this.clone())
        });

        methods.add_method("justify_between", |_, this, ()| {
            this.with_style_mut(|s| s.justify = Some(JustifyContent::SpaceBetween));
            Ok(this.clone())
        });

        methods.add_method("justify_start", |_, this, ()| {
            this.with_style_mut(|s| s.justify = Some(JustifyContent::FlexStart));
            Ok(this.clone())
        });

        methods.add_method("justify_end", |_, this, ()| {
            this.with_style_mut(|s| s.justify = Some(JustifyContent::FlexEnd));
            Ok(this.clone())
        });

        methods.add_method("justify_around", |_, this, ()| {
            this.with_style_mut(|s| s.justify = Some(JustifyContent::SpaceAround));
            Ok(this.clone())
        });

        methods.add_method("justify_evenly", |_, this, ()| {
            this.with_style_mut(|s| s.justify = Some(JustifyContent::SpaceEvenly));
            Ok(this.clone())
        });

        // Align Content
        methods.add_method("content_center", |_, this, ()| {
            this.with_style_mut(|s| s.align_content = Some(AlignContent::Center));
            Ok(this.clone())
        });

        methods.add_method("content_start", |_, this, ()| {
            this.with_style_mut(|s| s.align_content = Some(AlignContent::FlexStart));
            Ok(this.clone())
        });

        methods.add_method("content_end", |_, this, ()| {
            this.with_style_mut(|s| s.align_content = Some(AlignContent::FlexEnd));
            Ok(this.clone())
        });

        methods.add_method("content_between", |_, this, ()| {
            this.with_style_mut(|s| s.align_content = Some(AlignContent::SpaceBetween));
            Ok(this.clone())
        });

        methods.add_method("content_around", |_, this, ()| {
            this.with_style_mut(|s| s.align_content = Some(AlignContent::SpaceAround));
            Ok(this.clone())
        });

        methods.add_method("content_evenly", |_, this, ()| {
            this.with_style_mut(|s| s.align_content = Some(AlignContent::SpaceEvenly));
            Ok(this.clone())
        });

        methods.add_method("content_stretch", |_, this, ()| {
            this.with_style_mut(|s| s.align_content = Some(AlignContent::Stretch));
            Ok(this.clone())
        });

        // Gap
        methods.add_method("gap", |_, this, n: f32| {
            this.with_style_mut(|s| s.gap = Some(n));
            Ok(this.clone())
        });

        methods.add_method("gap_x", |_, this, n: f32| {
            this.with_style_mut(|s| s.gap_x = Some(n));
            Ok(this.clone())
        });

        methods.add_method("gap_y", |_, this, n: f32| {
            this.with_style_mut(|s| s.gap_y = Some(n));
            Ok(this.clone())
        });

        // Sizing & Aspect Ratio
        methods.add_method("aspect_ratio", |_, this, r: f32| {
            this.with_style_mut(|s| s.aspect_ratio = Some(r));
            Ok(this.clone())
        });

        methods.add_method("aspect_square", |_, this, ()| {
            this.with_style_mut(|s| s.aspect_square = true);
            Ok(this.clone())
        });

        methods.add_method("w", |_, this, val: Value| {
            let l = parse_length_value(val);
            this.with_style_mut(|s| s.width = l);
            Ok(this.clone())
        });

        methods.add_method("w_full", |_, this, ()| {
            this.with_style_mut(|s| s.width = Some(Length::Full));
            Ok(this.clone())
        });

        methods.add_method("h", |_, this, val: Value| {
            let l = parse_length_value(val);
            this.with_style_mut(|s| s.height = l);
            Ok(this.clone())
        });

        methods.add_method("h_full", |_, this, ()| {
            this.with_style_mut(|s| s.height = Some(Length::Full));
            Ok(this.clone())
        });

        methods.add_method("size", |_, this, (w, h): (Value, Option<Value>)| {
            let mut guard = this.inner.lock();
            match &mut *guard {
                BuilderKind::Text(t) => {
                    if let Value::Number(n) = w {
                        t.size = Some(n as f32);
                    } else if let Value::Integer(i) = w {
                        t.size = Some(i as f32);
                    }
                }
                BuilderKind::Div(d) => {
                    let lw = parse_length_value(w.clone());
                    let lh = if let Some(h_val) = h {
                        parse_length_value(h_val)
                    } else {
                        parse_length_value(w)
                    };
                    d.style.width = lw;
                    d.style.height = lh;
                }
                BuilderKind::Svg(s) => {
                    let lw = parse_length_value(w.clone());
                    let lh = if let Some(h_val) = h {
                        parse_length_value(h_val)
                    } else {
                        parse_length_value(w)
                    };
                    s.style.width = lw;
                    s.style.height = lh;
                }
                BuilderKind::Img(i) => {
                    let lw = parse_length_value(w.clone());
                    let lh = if let Some(h_val) = h {
                        parse_length_value(h_val)
                    } else {
                        parse_length_value(w)
                    };
                    i.style.width = lw;
                    i.style.height = lh;
                }
                BuilderKind::Canvas(c) => {
                    let lw = parse_length_value(w.clone());
                    let lh = if let Some(h_val) = h {
                        parse_length_value(h_val)
                    } else {
                        parse_length_value(w)
                    };
                    c.style.width = lw;
                    c.style.height = lh;
                }
                BuilderKind::Video(v) => {
                    let lw = parse_length_value(w.clone());
                    let lh = if let Some(h_val) = h {
                        parse_length_value(h_val)
                    } else {
                        parse_length_value(w)
                    };
                    v.style.width = lw;
                    v.style.height = lh;
                }
                BuilderKind::Input(inp) => {
                    let lw = parse_length_value(w.clone());
                    let lh = if let Some(h_val) = h {
                        parse_length_value(h_val)
                    } else {
                        parse_length_value(w)
                    };
                    inp.style.width = lw;
                    inp.style.height = lh;
                }
                BuilderKind::Custom(custom) => {
                    let lw = parse_length_value(w.clone());
                    let lh = if let Some(h_val) = h {
                        parse_length_value(h_val)
                    } else {
                        parse_length_value(w)
                    };
                    custom.style.width = lw;
                    custom.style.height = lh;
                }
            }
            drop(guard);
            Ok(this.clone())
        });

        methods.add_method("font_size", |_, this, size: f32| {
            this.with_text_mut(|t| t.size = Some(size));
            Ok(this.clone())
        });

        methods.add_method("min_w", |_, this, val: Value| {
            let l = parse_length_value(val);
            this.with_style_mut(|s| s.min_w = l);
            Ok(this.clone())
        });

        methods.add_method("min_h", |_, this, val: Value| {
            let l = parse_length_value(val);
            this.with_style_mut(|s| s.min_h = l);
            Ok(this.clone())
        });

        methods.add_method("max_w", |_, this, val: Value| {
            let l = parse_length_value(val);
            this.with_style_mut(|s| s.max_w = l);
            Ok(this.clone())
        });

        methods.add_method("max_h", |_, this, val: Value| {
            let l = parse_length_value(val);
            this.with_style_mut(|s| s.max_h = l);
            Ok(this.clone())
        });

        // Padding
        methods.add_method("p", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| s.padding = Edges::all(l));
            }
            Ok(this.clone())
        });

        methods.add_method("px", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| {
                    s.padding.left = Some(l);
                    s.padding.right = Some(l);
                });
            }
            Ok(this.clone())
        });

        methods.add_method("py", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| {
                    s.padding.top = Some(l);
                    s.padding.bottom = Some(l);
                });
            }
            Ok(this.clone())
        });

        methods.add_method("pt", |_, this, val: Value| {
            let l = parse_length_value(val);
            this.with_style_mut(|s| s.padding.top = l);
            Ok(this.clone())
        });

        methods.add_method("pr", |_, this, val: Value| {
            let l = parse_length_value(val);
            this.with_style_mut(|s| s.padding.right = l);
            Ok(this.clone())
        });

        methods.add_method("pb", |_, this, val: Value| {
            let l = parse_length_value(val);
            this.with_style_mut(|s| s.padding.bottom = l);
            Ok(this.clone())
        });

        methods.add_method("pl", |_, this, val: Value| {
            let l = parse_length_value(val);
            this.with_style_mut(|s| s.padding.left = l);
            Ok(this.clone())
        });

        // Margin
        methods.add_method("m", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| s.margin = Edges::all(l));
            }
            Ok(this.clone())
        });

        methods.add_method("mx", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| {
                    s.margin.left = Some(l);
                    s.margin.right = Some(l);
                });
            }
            Ok(this.clone())
        });

        methods.add_method("my", |_, this, val: Value| {
            if let Some(l) = parse_length_value(val) {
                this.with_style_mut(|s| {
                    s.margin.top = Some(l);
                    s.margin.bottom = Some(l);
                });
            }
            Ok(this.clone())
        });

        // Visuals & Filters
        methods.add_method("bg", |_, this, val: Value| {
            let color = parse_color_value(val);
            this.with_style_mut(|s| s.background = color);
            Ok(this.clone())
        });

        methods.add_method("text_color", |_, this, val: Value| {
            let color = parse_color_value(val);
            this.with_style_mut(|s| s.text_color = color.clone());
            this.with_text_mut(|t| t.color = color);
            Ok(this.clone())
        });

        methods.add_method("text_bg", |_, this, val: Value| {
            let color = parse_color_value(val);
            this.with_style_mut(|s| s.text_bg = color);
            Ok(this.clone())
        });

        methods.add_method("rounded", |_, this, r: f32| {
            this.with_style_mut(|s| s.corner_radius = Some(r));
            Ok(this.clone())
        });

        methods.add_method("corner_smoothing", |_, this, s: f32| {
            this.with_style_mut(|st| st.corner_smoothing = Some(s));
            Ok(this.clone())
        });

        methods.add_method("rounded_smoothing_ios", |_, this, ()| {
            this.with_style_mut(|s| s.rounded_smoothing_ios = true);
            Ok(this.clone())
        });

        methods.add_method("border", |_, this, b: f32| {
            this.with_style_mut(|s| s.border_width = Some(b));
            Ok(this.clone())
        });

        methods.add_method("border_t", |_, this, b: f32| {
            this.with_style_mut(|s| s.border_widths.top = Some(b));
            Ok(this.clone())
        });

        methods.add_method("border_b", |_, this, b: f32| {
            this.with_style_mut(|s| s.border_widths.bottom = Some(b));
            Ok(this.clone())
        });

        methods.add_method("border_l", |_, this, b: f32| {
            this.with_style_mut(|s| s.border_widths.left = Some(b));
            Ok(this.clone())
        });

        methods.add_method("border_r", |_, this, b: f32| {
            this.with_style_mut(|s| s.border_widths.right = Some(b));
            Ok(this.clone())
        });

        methods.add_method("border_x", |_, this, b: f32| {
            this.with_style_mut(|s| {
                s.border_widths.left = Some(b);
                s.border_widths.right = Some(b);
            });
            Ok(this.clone())
        });

        methods.add_method("border_y", |_, this, b: f32| {
            this.with_style_mut(|s| {
                s.border_widths.top = Some(b);
                s.border_widths.bottom = Some(b);
            });
            Ok(this.clone())
        });

        methods.add_method("rounded_t", |_, this, r: f32| {
            this.with_style_mut(|s| s.rounded_t = Some(r));
            Ok(this.clone())
        });

        methods.add_method("rounded_b", |_, this, r: f32| {
            this.with_style_mut(|s| s.rounded_b = Some(r));
            Ok(this.clone())
        });

        methods.add_method("rounded_l", |_, this, r: f32| {
            this.with_style_mut(|s| s.rounded_l = Some(r));
            Ok(this.clone())
        });

        methods.add_method("rounded_r", |_, this, r: f32| {
            this.with_style_mut(|s| s.rounded_r = Some(r));
            Ok(this.clone())
        });

        methods.add_method("rounded_tl", |_, this, r: f32| {
            this.with_style_mut(|s| s.rounded_tl = Some(r));
            Ok(this.clone())
        });

        methods.add_method("rounded_tr", |_, this, r: f32| {
            this.with_style_mut(|s| s.rounded_tr = Some(r));
            Ok(this.clone())
        });

        methods.add_method("rounded_bl", |_, this, r: f32| {
            this.with_style_mut(|s| s.rounded_bl = Some(r));
            Ok(this.clone())
        });

        methods.add_method("rounded_br", |_, this, r: f32| {
            this.with_style_mut(|s| s.rounded_br = Some(r));
            Ok(this.clone())
        });
        methods.add_method("border_color", |_, this, val: Value| {
            let color = parse_color_value(val);
            this.with_style_mut(|s| s.border_color = color);
            Ok(this.clone())
        });

        methods.add_method("border_dashed", |_, this, ()| {
            this.with_style_mut(|s| s.border_dashed = true);
            Ok(this.clone())
        });

        methods.add_method("border_dashed_length", |_, this, l: f32| {
            this.with_style_mut(|s| s.border_dashed_length = Some(l));
            Ok(this.clone())
        });

        methods.add_method("border_dashed_gap", |_, this, g: f32| {
            this.with_style_mut(|s| s.border_dashed_gap = Some(g));
            Ok(this.clone())
        });

        methods.add_method("shadow_sm", |_, this, ()| {
            this.with_style_mut(|s| s.shadow = Some(ShadowLevel::Sm));
            Ok(this.clone())
        });

        methods.add_method("shadow_md", |_, this, ()| {
            this.with_style_mut(|s| s.shadow = Some(ShadowLevel::Md));
            Ok(this.clone())
        });

        methods.add_method("shadow_lg", |_, this, ()| {
            this.with_style_mut(|s| s.shadow = Some(ShadowLevel::Lg));
            Ok(this.clone())
        });

        methods.add_method("opacity", |_, this, op: f32| {
            this.with_style_mut(|s| s.opacity = Some(op));
            Ok(this.clone())
        });

        methods.add_method("blur", |_, this, r: f32| {
            this.with_style_mut(|s| s.blur = Some(r));
            Ok(this.clone())
        });

        methods.add_method("backdrop_blur", |_, this, r: f32| {
            this.with_style_mut(|s| s.backdrop_blur = Some(r));
            Ok(this.clone())
        });

        // Overflow & Scrolling
        methods.add_method("overflow_hidden", |_, this, ()| {
            this.with_style_mut(|s| s.overflow_hidden = true);
            Ok(this.clone())
        });

        methods.add_method("overflow_scroll", |_, this, ()| {
            this.with_style_mut(|s| s.overflow_scroll = true);
            Ok(this.clone())
        });

        methods.add_method("overflow_fade", |_, this, val: Value| {
            match val {
                Value::Integer(i) => this.with_style_mut(|s| s.overflow_fade = Edges::all(i as f32)),
                Value::Number(n) => this.with_style_mut(|s| s.overflow_fade = Edges::all(n as f32)),
                Value::Table(tbl) => {
                    this.with_style_mut(|s| {
                        if let Ok(v) = tbl.get::<f32>("top").or_else(|_| tbl.get::<f32>("t")) {
                            s.overflow_fade.top = Some(v);
                        }
                        if let Ok(v) = tbl.get::<f32>("bottom").or_else(|_| tbl.get::<f32>("b")) {
                            s.overflow_fade.bottom = Some(v);
                        }
                        if let Ok(v) = tbl.get::<f32>("left").or_else(|_| tbl.get::<f32>("l")) {
                            s.overflow_fade.left = Some(v);
                        }
                        if let Ok(v) = tbl.get::<f32>("right").or_else(|_| tbl.get::<f32>("r")) {
                            s.overflow_fade.right = Some(v);
                        }
                    });
                }
                _ => {}
            }
            Ok(this.clone())
        });

        methods.add_method("overflow_fade_x", |_, this, f: f32| {
            this.with_style_mut(|s| {
                s.overflow_fade.left = Some(f);
                s.overflow_fade.right = Some(f);
            });
            Ok(this.clone())
        });

        methods.add_method("overflow_fade_y", |_, this, f: f32| {
            this.with_style_mut(|s| {
                s.overflow_fade.top = Some(f);
                s.overflow_fade.bottom = Some(f);
            });
            Ok(this.clone())
        });

        methods.add_method("overflow_fade_top", |_, this, f: f32| {
            this.with_style_mut(|s| s.overflow_fade.top = Some(f));
            Ok(this.clone())
        });

        methods.add_method("overflow_fade_t", |_, this, f: f32| {
            this.with_style_mut(|s| s.overflow_fade.top = Some(f));
            Ok(this.clone())
        });

        methods.add_method("overflow_fade_bottom", |_, this, f: f32| {
            this.with_style_mut(|s| s.overflow_fade.bottom = Some(f));
            Ok(this.clone())
        });

        methods.add_method("overflow_fade_b", |_, this, f: f32| {
            this.with_style_mut(|s| s.overflow_fade.bottom = Some(f));
            Ok(this.clone())
        });

        methods.add_method("overflow_fade_left", |_, this, f: f32| {
            this.with_style_mut(|s| s.overflow_fade.left = Some(f));
            Ok(this.clone())
        });

        methods.add_method("overflow_fade_l", |_, this, f: f32| {
            this.with_style_mut(|s| s.overflow_fade.left = Some(f));
            Ok(this.clone())
        });

        methods.add_method("overflow_fade_right", |_, this, f: f32| {
            this.with_style_mut(|s| s.overflow_fade.right = Some(f));
            Ok(this.clone())
        });

        methods.add_method("overflow_fade_r", |_, this, f: f32| {
            this.with_style_mut(|s| s.overflow_fade.right = Some(f));
            Ok(this.clone())
        });

        methods.add_method("scrollbar_width", |_, this, w: f32| {
            this.with_style_mut(|s| s.scrollbar_width = Some(w));
            Ok(this.clone())
        });

        // CSS Grid
        methods.add_method("grid", |_, this, ()| {
            this.with_div_mut(|d| d.style.grid = true);
            Ok(this.clone())
        });

        methods.add_method("grid_cols", |_, this, cols: u16| {
            this.with_div_mut(|d| {
                d.style.grid = true;
                d.style.grid_cols = Some(cols);
            });
            Ok(this.clone())
        });

        methods.add_method("grid_rows", |_, this, rows: u16| {
            this.with_div_mut(|d| {
                d.style.grid = true;
                d.style.grid_rows = Some(rows);
            });
            Ok(this.clone())
        });

        methods.add_method("col_span", |_, this, span: u16| {
            this.with_style_mut(|s| s.col_span = Some(span));
            Ok(this.clone())
        });

        methods.add_method("col_span_full", |_, this, ()| {
            this.with_style_mut(|s| s.col_span_full = true);
            Ok(this.clone())
        });

        methods.add_method("row_span", |_, this, span: u16| {
            this.with_style_mut(|s| s.row_span = Some(span));
            Ok(this.clone())
        });

        methods.add_method("row_span_full", |_, this, ()| {
            this.with_style_mut(|s| s.row_span_full = true);
            Ok(this.clone())
        });

        methods.add_method("col_start", |_, this, start: i16| {
            this.with_style_mut(|s| s.col_start = Some(start));
            Ok(this.clone())
        });

        methods.add_method("col_end", |_, this, end: i16| {
            this.with_style_mut(|s| s.col_end = Some(end));
            Ok(this.clone())
        });

        methods.add_method("row_start", |_, this, start: i16| {
            this.with_style_mut(|s| s.row_start = Some(start));
            Ok(this.clone())
        });

        methods.add_method("row_end", |_, this, end: i16| {
            this.with_style_mut(|s| s.row_end = Some(end));
            Ok(this.clone())
        });

        // Typography
        methods.add_method("text_left", |_, this, ()| {
            this.with_style_mut(|s| s.text_left = true);
            Ok(this.clone())
        });

        methods.add_method("text_center", |_, this, ()| {
            this.with_style_mut(|s| s.text_center = true);
            Ok(this.clone())
        });

        methods.add_method("text_right", |_, this, ()| {
            this.with_style_mut(|s| s.text_right = true);
            Ok(this.clone())
        });

        methods.add_method("truncate", |_, this, ()| {
            this.with_style_mut(|s| s.truncate = true);
            this.with_text_mut(|t| t.truncate = true);
            Ok(this.clone())
        });

        methods.add_method("line_clamp", |_, this, lines: usize| {
            this.with_style_mut(|s| s.line_clamp = Some(lines));
            this.with_text_mut(|t| t.line_clamp = Some(lines));
            Ok(this.clone())
        });

        methods.add_method("letter_spacing", |_, this, spacing: f32| {
            this.with_style_mut(|s| s.letter_spacing = Some(spacing));
            this.with_text_mut(|t| t.letter_spacing = Some(spacing));
            Ok(this.clone())
        });

        methods.add_method("underline", |_, this, ()| {
            this.with_style_mut(|s| s.underline = true);
            this.with_text_mut(|t| t.underline = true);
            Ok(this.clone())
        });

        methods.add_method("line_through", |_, this, ()| {
            this.with_style_mut(|s| s.line_through = true);
            this.with_text_mut(|t| t.line_through = true);
            Ok(this.clone())
        });

        methods.add_method("font_family", |_, this, name: String| {
            this.with_style_mut(|s| s.font_family = Some(name.clone()));
            this.with_text_mut(|t| t.font_family = Some(name));
            Ok(this.clone())
        });

        // Cursors
        methods.add_method("cursor_pointer", |_, this, ()| {
            this.with_style_mut(|s| s.cursor_pointer = true);
            Ok(this.clone())
        });

        methods.add_method("cursor", |_, this, cs_str: String| {
            if let Some(cs) = parse_cursor_style(&cs_str) {
                this.with_style_mut(|s| s.cursor = Some(cs));
            }
            Ok(this.clone())
        });

        methods.add_method("cursor_default", |_, this, ()| {
            this.with_style_mut(|s| s.cursor = Some(CursorStyle::Arrow));
            Ok(this.clone())
        });

        methods.add_method("cursor_text", |_, this, ()| {
            this.with_style_mut(|s| s.cursor = Some(CursorStyle::IBeam));
            Ok(this.clone())
        });

        methods.add_method("cursor_move", |_, this, ()| {
            this.with_style_mut(|s| s.cursor = Some(CursorStyle::ClosedHand));
            Ok(this.clone())
        });

        methods.add_method("cursor_not_allowed", |_, this, ()| {
            this.with_style_mut(|s| s.cursor = Some(CursorStyle::OperationNotAllowed));
            Ok(this.clone())
        });

        // Image fit
        methods.add_method("fit", |_, this, fit_str: String| {
            let fit = match fit_str.to_lowercase().as_str() {
                "contain" => Some(ImageFit::Contain),
                "cover" => Some(ImageFit::Cover),
                "fill" => Some(ImageFit::Fill),
                "none" => Some(ImageFit::None),
                "scale_down" => Some(ImageFit::ScaleDown),
                _ => None,
            };
            let mut guard = this.inner.lock();
            match &mut *guard {
                BuilderKind::Img(img) => img.fit = fit,
                BuilderKind::Video(v) => v.fit = fit,
                _ => {}
            }
            Ok(this.clone())
        });

        // Transitions
        methods.add_method("transition", |_, this, val: Value| {
            let mut trans = TransitionsProps::default();
            match val {
                Value::Integer(i) => trans.all = Some(i as u64),
                Value::Number(n) => trans.all = Some(n as u64),
                Value::Table(t) => {
                    if let Ok(i) = t.get::<u64>("all") {
                        trans.all = Some(i);
                    }
                    if let Ok(i) = t.get::<u64>("opacity") {
                        trans.opacity = Some(i);
                    }
                    if let Ok(i) = t.get::<u64>("bg").or_else(|_| t.get::<u64>("background")) {
                        trans.background = Some(i);
                    }
                    if let Ok(i) = t.get::<u64>("w").or_else(|_| t.get::<u64>("width")) {
                        trans.width = Some(i);
                    }
                    if let Ok(i) = t.get::<u64>("h").or_else(|_| t.get::<u64>("height")) {
                        trans.height = Some(i);
                    }
                    if let Ok(i) = t.get::<u64>("rounded").or_else(|_| t.get::<u64>("corner_radius")) {
                        trans.corner_radius = Some(i);
                    }
                    if let Ok(i) = t.get::<u64>("flex_grow") {
                        trans.flex_grow = Some(i);
                    }
                }
                _ => {}
            }
            this.with_div_mut(|d| d.transitions = Some(trans));
            Ok(this.clone())
        });

        // Animation
        methods.add_method(
            "animate",
            |_, this, (duration, repeat, easing): (u64, Option<bool>, Option<String>)| {
                this.with_div_mut(|d| {
                    d.animation = Some(AnimationProps {
                        duration_ms: duration,
                        repeat: repeat.unwrap_or(false),
                        easing,
                    });
                });
                Ok(this.clone())
            },
        );

        // Video Playback Controls
        methods.add_method("play", |lua, this, ()| {
            let src = match &*this.inner.lock() {
                BuilderKind::Video(v) => v.src.clone(),
                _ => return Ok(this.clone()),
            };
            if let Ok(play_fn) = lua.globals().get::<Function>("__gpui_video_play") {
                let _ = play_fn.call::<()>(src);
            }
            Ok(this.clone())
        });

        methods.add_method("pause", |lua, this, ()| {
            let src = match &*this.inner.lock() {
                BuilderKind::Video(v) => v.src.clone(),
                _ => return Ok(this.clone()),
            };
            if let Ok(pause_fn) = lua.globals().get::<Function>("__gpui_video_pause") {
                let _ = pause_fn.call::<()>(src);
            }
            Ok(this.clone())
        });

        methods.add_method("volume", |_, this, vol: f32| {
            if let BuilderKind::Video(v) = &mut *this.inner.lock() {
                v.volume = vol;
            }
            Ok(this.clone())
        });

        methods.add_method("loop", |_, this, looping: Option<bool>| {
            if let BuilderKind::Video(v) = &mut *this.inner.lock() {
                v.looping = looping.unwrap_or(true);
            }
            Ok(this.clone())
        });

        methods.add_method("muted", |_, this, muted: Option<bool>| {
            if let BuilderKind::Video(v) = &mut *this.inner.lock() {
                v.muted = muted.unwrap_or(true);
            }
            Ok(this.clone())
        });

        // Children and event methods
        methods.add_method("child", |lua, this, val: Value| {
            if let Some(node) = value_to_node(lua, val) {
                this.with_div_mut(|d| d.children.push(node.clone()));
                this.with_custom_mut(|c| c.children.push(node));
            }
            Ok(this.clone())
        });

        methods.add_method("children", |lua, this, table: Table| {
            for v in table.sequence_values::<Value>() {
                if let Ok(val) = v {
                    if let Some(node) = value_to_node(lua, val) {
                        this.with_div_mut(|d| d.children.push(node.clone()));
                        this.with_custom_mut(|c| c.children.push(node));
                    }
                }
            }
            Ok(this.clone())
        });

        methods.add_method("prop", |_lua, this, (key, val): (String, Value)| {
            let json_val = crate::dsl::custom::lua_value_to_json(&val);
            this.with_custom_mut(|c| {
                if let serde_json::Value::Object(ref mut map) = c.props {
                    map.insert(key, json_val);
                }
            });
            Ok(this.clone())
        });

        methods.add_method("on_click", |lua, this, func: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(func)?);
            this.with_div_mut(|d| d.on_click = Some(key));
            Ok(this.clone())
        });

        methods.add_method("on_hover", |lua, this, func: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(func)?);
            this.with_div_mut(|d| d.on_hover = Some(key));
            Ok(this.clone())
        });

        methods.add_method("on_drop", |lua, this, func: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(func)?);
            this.with_div_mut(|d| d.on_drop = Some(key));
            Ok(this.clone())
        });

        methods.add_method("on_mouse_down", |lua, this, func: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(func)?);
            this.with_div_mut(|d| d.on_mouse_down = Some(key));
            Ok(this.clone())
        });

        methods.add_method("on_mouse_up", |lua, this, func: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(func)?);
            this.with_div_mut(|d| d.on_mouse_up = Some(key));
            Ok(this.clone())
        });

        methods.add_method("on_mouse_move", |lua, this, func: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(func)?);
            this.with_div_mut(|d| d.on_mouse_move = Some(key));
            Ok(this.clone())
        });

        methods.add_method("on_key_down", |lua, this, func: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(func)?);
            this.with_div_mut(|d| d.on_key_down = Some(key));
            Ok(this.clone())
        });

        methods.add_method("on_key_up", |lua, this, func: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(func)?);
            this.with_div_mut(|d| d.on_key_up = Some(key));
            Ok(this.clone())
        });

        methods.add_method("on_scroll_wheel", |lua, this, func: Function| {
            let key = SyncRegistryKey::new(lua.create_registry_value(func)?);
            this.with_div_mut(|d| d.on_scroll_wheel = Some(key));
            Ok(this.clone())
        });
        methods.add_method("focusable", |_, this, val: Option<bool>| {
            this.with_div_mut(|d| d.focusable = val.unwrap_or(true));
            Ok(this.clone())
        });

        methods.add_method("tab_index", |_, this, idx: isize| {
            this.with_div_mut(|d| {
                d.focusable = true;
                d.tab_index = Some(idx);
            });
            Ok(this.clone())
        });
        // Window control area methods
        methods.add_method("window_control", |_, this, area: String| {
            let ctrl = match area.to_lowercase().as_str() {
                "drag" => Some(gpui::WindowControlArea::Drag),
                "close" => Some(gpui::WindowControlArea::Close),
                "max" | "maximize" => Some(gpui::WindowControlArea::Max),
                "min" | "minimize" => Some(gpui::WindowControlArea::Min),
                _ => None,
            };
            this.with_div_mut(|d| d.window_control = ctrl);
            Ok(this.clone())
        });

        methods.add_method("drag_area", |_, this, ()| {
            this.with_div_mut(|d| d.window_control = Some(gpui::WindowControlArea::Drag));
            Ok(this.clone())
        });

        methods.add_method("window_close", |_, this, ()| {
            this.with_div_mut(|d| d.window_control = Some(gpui::WindowControlArea::Close));
            Ok(this.clone())
        });

        methods.add_method("window_max", |_, this, ()| {
            this.with_div_mut(|d| d.window_control = Some(gpui::WindowControlArea::Max));
            Ok(this.clone())
        });

        methods.add_method("window_min", |_, this, ()| {
            this.with_div_mut(|d| d.window_control = Some(gpui::WindowControlArea::Min));
            Ok(this.clone())
        });

        methods.add_method("stop_propagation", |_, this, val: Option<bool>| {
            this.with_div_mut(|d| d.stop_propagation = val.unwrap_or(true));
            Ok(this.clone())
        });

        // Text specific methods
        // Text font_size also available via :size(n) above

        methods.add_method("color", |_, this, val: Value| {
            let color = parse_color_value(val);
            this.with_text_mut(|t| t.color = color.clone());
            this.with_style_mut(|s| s.text_color = color);
            Ok(this.clone())
        });

        methods.add_method("bold", |_, this, ()| {
            this.with_text_mut(|t| t.bold = true);
            Ok(this.clone())
        });

        methods.add_method("italic", |_, this, ()| {
            this.with_text_mut(|t| t.italic = true);
            Ok(this.clone())
        });

        methods.add_method("line_height", |_, this, lh: f32| {
            this.with_text_mut(|t| t.line_height = Some(lh));
            Ok(this.clone())
        });

        methods.add_method("rows", |_, this, r: usize| {
            this.with_input_mut(|i| i.rows = r);
            Ok(this.clone())
        });

        methods.add_method("max_rows", |_, this, r: usize| {
            this.with_input_mut(|i| i.max_rows = Some(r));
            Ok(this.clone())
        });
    }
}
