use gpui::{
    AlignContent, AlignItems, AlignSelf, CursorStyle, Display, FlexDirection, Hsla, JustifyContent,
    Rgba, WindowControlArea, rgb_to_hsla,
};
use mlua::RegistryKey;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct SyncRegistryKey(pub Arc<Mutex<RegistryKey>>);

impl SyncRegistryKey {
    pub fn new(key: RegistryKey) -> Self {
        Self(Arc::new(Mutex::new(key)))
    }

    pub fn with<R>(&self, f: impl FnOnce(&RegistryKey) -> R) -> R {
        let guard = self.0.lock();
        f(&guard)
    }
}

impl PartialEq for SyncRegistryKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum ColorSpec {
    Hex(String),
    Rgb(u8, u8, u8),
    Rgba(u8, u8, u8, f32),
}

impl ColorSpec {
    pub fn to_hsla(&self) -> Hsla {
        match self {
            ColorSpec::Hex(hex_str) => parse_hex_color(hex_str),
            ColorSpec::Rgb(r, g, b) => {
                let rgba = Rgba::new(*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0, 1.0);
                rgb_to_hsla(rgba)
            }
            ColorSpec::Rgba(r, g, b, a) => {
                let rgba = Rgba::new(*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0, *a);
                rgb_to_hsla(rgba)
            }
        }
    }
}

pub fn parse_hex_color(s: &str) -> Hsla {
    let clean = s.trim().trim_start_matches('#');
    let (r, g, b, a) = match clean.len() {
        3 => {
            let r = u8::from_str_radix(&clean[0..1].repeat(2), 16).unwrap_or(0);
            let g = u8::from_str_radix(&clean[1..2].repeat(2), 16).unwrap_or(0);
            let b = u8::from_str_radix(&clean[2..3].repeat(2), 16).unwrap_or(0);
            (r, g, b, 1.0f32)
        }
        4 => {
            let r = u8::from_str_radix(&clean[0..1].repeat(2), 16).unwrap_or(0);
            let g = u8::from_str_radix(&clean[1..2].repeat(2), 16).unwrap_or(0);
            let b = u8::from_str_radix(&clean[2..3].repeat(2), 16).unwrap_or(0);
            let a = u8::from_str_radix(&clean[3..4].repeat(2), 16).unwrap_or(255) as f32 / 255.0;
            (r, g, b, a)
        }
        6 => {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0);
            (r, g, b, 1.0f32)
        }
        8 => {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0);
            let a = u8::from_str_radix(&clean[6..8], 16).unwrap_or(255) as f32 / 255.0;
            (r, g, b, a)
        }
        _ => match clean.to_lowercase().as_str() {
            "transparent" => (0, 0, 0, 0.0),
            "black" => (0, 0, 0, 1.0),
            "white" => (255, 255, 255, 1.0),
            "red" => (255, 0, 0, 1.0),
            "green" => (0, 255, 0, 1.0),
            "blue" => (0, 0, 255, 1.0),
            _ => (0, 0, 0, 1.0),
        },
    };

    let rgba = Rgba::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a);
    rgb_to_hsla(rgba)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    Px(f32),
    Percent(f32),
    Auto,
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edges<T> {
    pub top: Option<T>,
    pub right: Option<T>,
    pub bottom: Option<T>,
    pub left: Option<T>,
}

impl<T> Default for Edges<T> {
    fn default() -> Self {
        Self {
            top: None,
            right: None,
            bottom: None,
            left: None,
        }
    }
}

impl<T: Copy> Edges<T> {
    pub fn all(val: T) -> Self {
        Self {
            top: Some(val),
            right: Some(val),
            bottom: Some(val),
            left: Some(val),
        }
    }

    pub fn x(val: T) -> Self {
        Self {
            top: None,
            right: Some(val),
            bottom: None,
            left: Some(val),
        }
    }

    pub fn y(val: T) -> Self {
        Self {
            top: Some(val),
            right: None,
            bottom: Some(val),
            left: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShadowLevel {
    Sm,
    Md,
    Lg,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleProps {
    // Layout & Positioning
    pub display: Option<Display>,
    pub position: Option<gpui::Position>,
    pub inset: Edges<Length>,
    pub visibility: Option<gpui::Visibility>,
    pub flex: Option<FlexDirection>,
    pub flex_wrap: bool,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub flex_basis: Option<Length>,
    pub flex_1: bool,
    pub flex_auto: bool,
    pub flex_none: bool,
    pub items: Option<AlignItems>,
    pub align_self: Option<AlignSelf>,
    pub align_content: Option<AlignContent>,
    pub justify: Option<JustifyContent>,
    pub gap: Option<f32>,
    pub gap_x: Option<f32>,
    pub gap_y: Option<f32>,
    // Sizing
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub min_w: Option<Length>,
    pub min_h: Option<Length>,
    pub max_w: Option<Length>,
    pub max_h: Option<Length>,
    pub aspect_ratio: Option<f32>,
    pub aspect_square: bool,

    // Spacing
    pub padding: Edges<Length>,
    pub margin: Edges<Length>,

    // Visuals & Filters
    pub background: Option<ColorSpec>,
    pub text_color: Option<ColorSpec>,
    pub text_bg: Option<ColorSpec>,
    pub border_width: Option<f32>,
    pub border_widths: Edges<f32>,
    pub border_color: Option<ColorSpec>,
    pub border_dashed: bool,
    pub border_dashed_length: Option<f32>,
    pub border_dashed_gap: Option<f32>,
    pub corner_radius: Option<f32>,
    pub rounded_t: Option<f32>,
    pub rounded_b: Option<f32>,
    pub rounded_l: Option<f32>,
    pub rounded_r: Option<f32>,
    pub rounded_tl: Option<f32>,
    pub rounded_tr: Option<f32>,
    pub rounded_br: Option<f32>,
    pub rounded_bl: Option<f32>,
    pub corner_smoothing: Option<f32>,
    pub rounded_smoothing_ios: bool,
    pub shadow: Option<ShadowLevel>,
    pub opacity: Option<f32>,
    pub blur: Option<f32>,
    pub backdrop_blur: Option<f32>,
    pub cursor_pointer: bool,
    pub cursor: Option<CursorStyle>,

    // Overflow & Scrolling
    pub overflow_hidden: bool,
    pub overflow_scroll: bool,
    pub overflow_fade: Edges<f32>,
    pub scrollbar_width: Option<f32>,

    // Grid
    pub grid: bool,
    pub grid_cols: Option<u16>,
    pub grid_rows: Option<u16>,
    pub col_start: Option<i16>,
    pub col_end: Option<i16>,
    pub col_span: Option<u16>,
    pub col_span_full: bool,
    pub row_start: Option<i16>,
    pub row_end: Option<i16>,
    pub row_span: Option<u16>,
    pub row_span_full: bool,

    // Typography
    pub text_left: bool,
    pub text_center: bool,
    pub text_right: bool,
    pub letter_spacing: Option<f32>,
    pub truncate: bool,
    pub line_clamp: Option<usize>,
    pub underline: bool,
    pub line_through: bool,
    pub font_family: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TransitionsProps {
    pub all: Option<u64>,
    pub opacity: Option<u64>,
    pub background: Option<u64>,
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub corner_radius: Option<u64>,
    pub flex_grow: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimationProps {
    pub duration_ms: u64,
    pub repeat: bool,
    pub easing: Option<String>,
}

#[derive(Clone, Debug)]
pub enum LuaNode {
    Div(DivNode),
    Text(TextNode),
    Svg(SvgNode),
    Img(ImgNode),
    Canvas(CanvasNode),
    Video(VideoNode),
    Input(InputNode),
    Custom(CustomNode),
}

#[derive(Clone, Debug, Default)]
pub struct CustomNode {
    pub tag: String,
    pub props: serde_json::Value,
    pub children: Vec<LuaNode>,
    pub style: StyleProps,
}
#[derive(Clone, Debug, Default)]
pub struct InputNode {
    pub id: String,
    pub value: Option<String>,
    pub placeholder: String,
    pub placeholder_color: Option<ColorSpec>,
    pub mask: Option<char>,
    pub multiline: bool,
    pub rows: usize,
    pub max_rows: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub required: bool,
    pub error_message: Option<String>,
    pub disabled: bool,
    pub read_only: bool,
    pub autofocus: bool,
    pub font_size: Option<f32>,
    pub style: StyleProps,
    pub focus_border_color: Option<ColorSpec>,
    pub focus_bg: Option<ColorSpec>,
    pub error_border_color: Option<ColorSpec>,
    pub cursor_color: Option<ColorSpec>,
    pub selection_bg: Option<ColorSpec>,
    pub on_change: Option<SyncRegistryKey>,
    pub on_input: Option<SyncRegistryKey>,
    pub on_submit: Option<SyncRegistryKey>,
    pub on_focus: Option<SyncRegistryKey>,
    pub on_blur: Option<SyncRegistryKey>,
    pub on_validate: Option<SyncRegistryKey>,
}

#[derive(Clone, Debug, Default)]
pub struct DivNode {
    pub id: Option<String>,
    pub style: StyleProps,
    pub children: Vec<LuaNode>,
    pub on_click: Option<SyncRegistryKey>,
    pub on_mouse_down: Option<SyncRegistryKey>,
    pub on_mouse_up: Option<SyncRegistryKey>,
    pub on_mouse_move: Option<SyncRegistryKey>,
    pub on_mouse_enter: Option<SyncRegistryKey>,
    pub on_mouse_leave: Option<SyncRegistryKey>,
    pub on_hover: Option<SyncRegistryKey>,
    pub on_drag: Option<SyncRegistryKey>,
    pub on_drop: Option<SyncRegistryKey>,
    pub on_scroll_wheel: Option<SyncRegistryKey>,
    pub on_key_down: Option<SyncRegistryKey>,
    pub on_key_up: Option<SyncRegistryKey>,
    pub transitions: Option<TransitionsProps>,
    pub animation: Option<AnimationProps>,
    pub window_control: Option<WindowControlArea>,
    pub stop_propagation: bool,
    pub focusable: bool,
    pub tab_index: Option<isize>,
}

#[derive(Clone, Debug, Default)]
pub struct SvgNode {
    pub id: Option<String>,
    pub path: Option<String>,
    pub data: Option<String>,
    pub style: StyleProps,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImageFit {
    Contain,
    Cover,
    Fill,
    None,
    ScaleDown,
}

impl ImageFit {
    pub fn to_gpui(&self) -> gpui::ObjectFit {
        match self {
            ImageFit::Contain => gpui::ObjectFit::Contain,
            ImageFit::Cover => gpui::ObjectFit::Cover,
            ImageFit::Fill => gpui::ObjectFit::Fill,
            ImageFit::None => gpui::ObjectFit::None,
            ImageFit::ScaleDown => gpui::ObjectFit::ScaleDown,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ImgNode {
    pub id: Option<String>,
    pub src: String,
    pub fit: Option<ImageFit>,
    pub style: StyleProps,
}

#[derive(Clone, Debug, Default)]
pub struct CanvasNode {
    pub id: Option<String>,
    pub style: StyleProps,
    pub paint: Option<SyncRegistryKey>,
}
#[derive(Clone, Debug, Default)]
pub struct TextNode {
    pub content: String,
    pub size: Option<f32>,
    pub color: Option<ColorSpec>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub line_through: bool,
    pub line_height: Option<f32>,
    pub font_family: Option<String>,
    pub letter_spacing: Option<f32>,
    pub truncate: bool,
    pub line_clamp: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct VideoNode {
    pub id: Option<String>,
    pub src: String,
    pub fit: Option<ImageFit>,
    pub autoplay: bool,
    pub looping: bool,
    pub muted: bool,
    pub volume: f32,
    pub style: StyleProps,
}

impl Default for VideoNode {
    fn default() -> Self {
        Self {
            id: None,
            src: String::new(),
            fit: None,
            autoplay: true,
            looping: false,
            muted: false,
            volume: 1.0,
            style: StyleProps::default(),
        }
    }
}

