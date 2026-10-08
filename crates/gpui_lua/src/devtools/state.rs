use crate::dsl::node::{ColorSpec, Length, LuaNode, StyleProps};
use gpui::{Bounds, Pixels};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevToolsTab {
    Elements,
    Network,
    Storage,
    Console,
    Performance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementsSubTab {
    Styles,
    BoxModel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageSubTab {
    LocalStorage,
    Databases,
    Signals,
}

#[derive(Clone, Debug)]
pub struct ElementTreeNode {
    pub path: Vec<usize>,
    pub tag: String,
    pub id: Option<String>,
    pub text_preview: Option<String>,
    pub attrs: Vec<(String, String)>,
    pub children_count: usize,
    pub style: StyleProps,
    pub children: Vec<ElementTreeNode>,
}

impl ElementTreeNode {
    pub fn from_lua_node(node: &LuaNode, path: Vec<usize>) -> Self {
        match node {
            LuaNode::Div(d) => {
                let tag = if let Some(ref id) = d.id {
                    if id.starts_with("btn_") || id.contains("button") {
                        "button".to_string()
                    } else if id.starts_with("card_") || id.contains("card") {
                        "card".to_string()
                    } else if matches!(d.style.flex, Some(gpui::FlexDirection::Row)) {
                        "row".to_string()
                    } else {
                        "div".to_string()
                    }
                } else if matches!(d.style.flex, Some(gpui::FlexDirection::Row)) {
                    "row".to_string()
                } else {
                    "div".to_string()
                };

                let mut attrs = Vec::new();
                if let Some(ref w) = d.style.width {
                    match w {
                        Length::Px(p) => attrs.push(("w".to_string(), format!("{p:.0}"))),
                        Length::Percent(pct) => attrs.push(("w".to_string(), format!("{pct:.0}%"))),
                        Length::Full => attrs.push(("w".to_string(), "full".to_string())),
                        _ => {}
                    }
                }
                if let Some(ref h) = d.style.height {
                    match h {
                        Length::Px(p) => attrs.push(("h".to_string(), format!("{p:.0}"))),
                        Length::Percent(pct) => attrs.push(("h".to_string(), format!("{pct:.0}%"))),
                        Length::Full => attrs.push(("h".to_string(), "full".to_string())),
                        _ => {}
                    }
                }
                if let Some(ref bg) = d.style.background {
                    match bg {
                        ColorSpec::Hex(hex) => attrs.push(("bg".to_string(), hex.clone())),
                        _ => {}
                    }
                }
                // Distinguish all padding edges: p, px, py, pt, pr, pb, pl
                push_edge_attrs(&mut attrs, "p", &d.style.padding);

                // Distinguish all margin edges: m, mx, my, mt, mr, mb, ml
                push_edge_attrs(&mut attrs, "m", &d.style.margin);

                // Distinguish border
                if let Some(bw) = d.style.border_width {
                    attrs.push(("border".to_string(), format!("{bw:.0}")));
                } else {
                    let b = &d.style.border_widths;
                    if b.top == b.bottom && b.left == b.right && b.top == b.left && b.top.unwrap_or(0.0) > 0.0 {
                        attrs.push(("border".to_string(), format!("{:.0}", b.top.unwrap_or(0.0))));
                    } else {
                        if let Some(t) = b.top { if t > 0.0 { attrs.push(("border_t".to_string(), format!("{t:.0}"))); } }
                        if let Some(r) = b.right { if r > 0.0 { attrs.push(("border_r".to_string(), format!("{r:.0}"))); } }
                        if let Some(bot) = b.bottom { if bot > 0.0 { attrs.push(("border_b".to_string(), format!("{bot:.0}"))); } }
                        if let Some(l) = b.left { if l > 0.0 { attrs.push(("border_l".to_string(), format!("{l:.0}"))); } }
                    }
                }
                if let Some(r) = d.style.corner_radius {
                    attrs.push(("rounded".to_string(), format!("{r:.0}")));
                }
                if let Some(g) = d.style.gap {
                    attrs.push(("gap".to_string(), format!("{g:.0}")));
                }

                let children: Vec<ElementTreeNode> = d
                    .children
                    .iter()
                    .enumerate()
                    .map(|(i, ch)| {
                        let mut p = path.clone();
                        p.push(i);
                        ElementTreeNode::from_lua_node(ch, p)
                    })
                    .collect();

                let children_count = children.len();
                Self {
                    path,
                    tag,
                    id: d.id.clone(),
                    text_preview: None,
                    attrs,
                    children_count,
                    style: d.style.clone(),
                    children,
                }
            }
            LuaNode::Text(t) => {
                let preview = if t.content.len() > 30 {
                    format!("{}...", &t.content[..30])
                } else {
                    t.content.clone()
                };

                let mut attrs = Vec::new();
                if let Some(sz) = t.size {
                    attrs.push(("size".to_string(), format!("{sz:.0}")));
                }
                if t.bold {
                    attrs.push(("bold".to_string(), "true".to_string()));
                }
                if let Some(ref c) = t.color {
                    match c {
                        ColorSpec::Hex(h) => attrs.push(("color".to_string(), h.clone())),
                        _ => {}
                    }
                }

                let mut style = StyleProps::default();
                style.font_size = t.size;
                if let Some(c) = &t.color {
                    style.text_color = Some(c.clone());
                }
                if let Some(ff) = &t.font_family {
                    style.font_family = Some(ff.clone());
                }

                Self {
                    path,
                    tag: "text".to_string(),
                    id: None,
                    text_preview: Some(preview),
                    attrs,
                    children_count: 0,
                    style,
                    children: Vec::new(),
                }
            }
            LuaNode::Input(inp) => Self {
                path,
                tag: if inp.multiline { "textarea".to_string() } else { "input".to_string() },
                id: Some(inp.id.clone()),
                text_preview: inp.value.clone().or_else(|| Some(inp.placeholder.clone())),
                attrs: vec![("type".to_string(), if inp.multiline { "textarea".to_string() } else { "text".to_string() })],
                children_count: 0,
                style: inp.style.clone(),
                children: Vec::new(),
            },
            LuaNode::Custom(c) => {
                let children: Vec<ElementTreeNode> = c
                    .children
                    .iter()
                    .enumerate()
                    .map(|(i, ch)| {
                        let mut p = path.clone();
                        p.push(i);
                        ElementTreeNode::from_lua_node(ch, p)
                    })
                    .collect();

                let mut attrs = Vec::new();
                if let serde_json::Value::Object(ref map) = c.props {
                    for (k, v) in map.iter().take(3) {
                        let val_str = match v {
                            serde_json::Value::String(s) => s.clone(),
                            _ => v.to_string(),
                        };
                        attrs.push((k.clone(), val_str));
                    }
                }

                Self {
                    path,
                    tag: c.tag.clone(),
                    id: None,
                    text_preview: None,
                    attrs,
                    children_count: children.len(),
                    style: c.style.clone(),
                    children,
                }
            }
            LuaNode::Svg(s) => Self {
                path,
                tag: "svg".to_string(),
                id: s.id.clone(),
                text_preview: s.path.clone(),
                attrs: Vec::new(),
                children_count: 0,
                style: s.style.clone(),
                children: Vec::new(),
            },
            LuaNode::Img(im) => Self {
                path,
                tag: "img".to_string(),
                id: im.id.clone(),
                text_preview: Some(im.src.clone()),
                attrs: vec![("src".to_string(), im.src.clone())],
                children_count: 0,
                style: im.style.clone(),
                children: Vec::new(),
            },
            LuaNode::Video(v) => Self {
                path,
                tag: "video".to_string(),
                id: v.id.clone(),
                text_preview: Some(v.src.clone()),
                attrs: vec![("src".to_string(), v.src.clone())],
                children_count: 0,
                style: v.style.clone(),
                children: Vec::new(),
            },
            LuaNode::Canvas(c) => Self {
                path,
                tag: "canvas".to_string(),
                id: c.id.clone(),
                text_preview: None,
                attrs: Vec::new(),
                children_count: 0,
                style: c.style.clone(),
                children: Vec::new(),
            },
        }
    }

    pub fn find_by_path(&self, target: &[usize]) -> Option<&ElementTreeNode> {
        if self.path == target {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_path(target) {
                return Some(found);
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BoxModelMetrics {
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub border_top: f32,
    pub border_right: f32,
    pub border_bottom: f32,
    pub border_left: f32,
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    pub content_width: f32,
    pub content_height: f32,
}

impl BoxModelMetrics {
    pub fn from_style_and_bounds(style: &StyleProps, bounds: Bounds<Pixels>) -> Self {
        let (pt, pr, pb, pl) = (
            style.padding.top.map(|l| match l { Length::Px(v) => v, _ => 0.0 }).unwrap_or(0.0),
            style.padding.right.map(|l| match l { Length::Px(v) => v, _ => 0.0 }).unwrap_or(0.0),
            style.padding.bottom.map(|l| match l { Length::Px(v) => v, _ => 0.0 }).unwrap_or(0.0),
            style.padding.left.map(|l| match l { Length::Px(v) => v, _ => 0.0 }).unwrap_or(0.0),
        );

        let (mt, mr, mb, ml) = (
            style.margin.top.map(|l| match l { Length::Px(v) => v, _ => 0.0 }).unwrap_or(0.0),
            style.margin.right.map(|l| match l { Length::Px(v) => v, _ => 0.0 }).unwrap_or(0.0),
            style.margin.bottom.map(|l| match l { Length::Px(v) => v, _ => 0.0 }).unwrap_or(0.0),
            style.margin.left.map(|l| match l { Length::Px(v) => v, _ => 0.0 }).unwrap_or(0.0),
        );

        let default_border = style.border_width.unwrap_or(0.0);
        let (bt, br, bb, bl) = (
            style.border_widths.top.unwrap_or(default_border),
            style.border_widths.right.unwrap_or(default_border),
            style.border_widths.bottom.unwrap_or(default_border),
            style.border_widths.left.unwrap_or(default_border),
        );

        let total_w: f32 = bounds.size.width.into();
        let total_h: f32 = bounds.size.height.into();

        let content_w = (total_w - pl - pr - bl - br).max(0.0);
        let content_h = (total_h - pt - pb - bt - bb).max(0.0);

        Self {
            margin_top: mt,
            margin_right: mr,
            margin_bottom: mb,
            margin_left: ml,
            border_top: bt,
            border_right: br,
            border_bottom: bb,
            border_left: bl,
            padding_top: pt,
            padding_right: pr,
            padding_bottom: pb,
            padding_left: pl,
            content_width: content_w,
            content_height: content_h,
        }
    }
}

fn push_edge_attrs(attrs: &mut Vec<(String, String)>, prefix: &str, edges: &crate::dsl::node::Edges<crate::dsl::node::Length>) {
    use crate::dsl::node::Length;
    let t = edges.top.as_ref();
    let r = edges.right.as_ref();
    let b = edges.bottom.as_ref();
    let l = edges.left.as_ref();

    let fmt_len = |len: &Length| -> String {
        match len {
            Length::Px(v) => format!("{v:.0}"),
            Length::Percent(p) => format!("{p:.0}%"),
            Length::Full => "full".to_string(),
            Length::Auto => "auto".to_string(),
        }
    };

    // If all 4 edges are set and equal
    if t.is_some() && t == r && t == b && t == l {
        attrs.push((prefix.to_string(), fmt_len(t.unwrap())));
        return;
    }

    let x_equal = l.is_some() && l == r;
    let y_equal = t.is_some() && t == b;

    if x_equal {
        attrs.push((format!("{prefix}x"), fmt_len(l.unwrap())));
    } else {
        if let Some(pl) = l {
            attrs.push((format!("{prefix}l"), fmt_len(pl)));
        }
        if let Some(pr) = r {
            attrs.push((format!("{prefix}r"), fmt_len(pr)));
        }
    }

    if y_equal {
        attrs.push((format!("{prefix}y"), fmt_len(t.unwrap())));
    } else {
        if let Some(pt) = t {
            attrs.push((format!("{prefix}t"), fmt_len(pt)));
        }
        if let Some(pb) = b {
            attrs.push((format!("{prefix}b"), fmt_len(pb)));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkType {
    Fetch,
    WebSocket,
    WebRtc,
}

#[derive(Clone, Debug)]
pub struct WsFrameLog {
    pub is_outgoing: bool,
    pub timestamp: String,
    pub payload: String,
    pub size_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct NetworkEntry {
    pub id: u64,
    pub entry_type: NetworkType,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub status_text: String,
    pub start_time: Instant,
    pub start_time_str: String,
    pub duration_ms: Option<f64>,
    pub request_headers: HashMap<String, String>,
    pub request_body: Option<String>,
    pub response_headers: HashMap<String, String>,
    pub response_body: Option<String>,
    pub response_size: usize,
    pub ws_frames: Vec<WsFrameLog>,
    pub webrtc_stats: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

#[derive(Clone, Debug)]
pub struct ConsoleEntry {
    pub id: u64,
    pub level: LogLevel,
    pub time_str: String,
    pub message: String,
}

pub struct DevToolsState {
    pub enabled: AtomicBool,
    pub active_tab: RwLock<DevToolsTab>,
    pub elements_sub_tab: RwLock<ElementsSubTab>,
    pub storage_sub_tab: RwLock<StorageSubTab>,
    pub inspect_cursor_active: AtomicBool,
    pub selected_path: RwLock<Option<Vec<usize>>>,
    pub hovered_path: RwLock<Option<Vec<usize>>>,
    pub node_bounds: RwLock<HashMap<Vec<usize>, Bounds<Pixels>>>,
    pub element_tree: RwLock<Option<ElementTreeNode>>,
    pub network_entries: RwLock<Vec<NetworkEntry>>,
    pub selected_network_id: RwLock<Option<u64>>,
    pub network_filter: RwLock<Option<NetworkType>>,
    pub network_search: RwLock<String>,
    pub console_entries: RwLock<Vec<ConsoleEntry>>,
    pub console_filter: RwLock<Option<LogLevel>>,
    pub console_input: RwLock<String>,
    pub sql_query: RwLock<String>,
    pub sql_query_result: RwLock<Option<Result<Vec<HashMap<String, serde_json::Value>>, String>>>,
    pub fps: RwLock<f32>,
    pub frame_time_ms: RwLock<f32>,
    pub collapsed_paths: RwLock<std::collections::HashSet<Vec<usize>>>,
    pub next_id: AtomicU64,
    pub bridge: RwLock<Option<crate::reactive::bridge::ReactiveBridge>>,
}
impl Default for DevToolsState {
    fn default() -> Self {
        Self {
            enabled: AtomicBool::new(cfg!(debug_assertions)),
            active_tab: RwLock::new(DevToolsTab::Elements),
            elements_sub_tab: RwLock::new(ElementsSubTab::Styles),
            storage_sub_tab: RwLock::new(StorageSubTab::LocalStorage),
            inspect_cursor_active: AtomicBool::new(false),
            selected_path: RwLock::new(None),
            hovered_path: RwLock::new(None),
            node_bounds: RwLock::new(HashMap::new()),
            element_tree: RwLock::new(None),
            network_entries: RwLock::new(Vec::new()),
            selected_network_id: RwLock::new(None),
            network_filter: RwLock::new(None),
            network_search: RwLock::new(String::new()),
            console_entries: RwLock::new(Vec::new()),
            console_filter: RwLock::new(None),
            console_input: RwLock::new(String::new()),
            sql_query: RwLock::new("SELECT name FROM sqlite_master WHERE type='table';".to_string()),
            sql_query_result: RwLock::new(None),
            fps: RwLock::new(60.0),
            frame_time_ms: RwLock::new(16.6),
            collapsed_paths: RwLock::new(std::collections::HashSet::new()),
            next_id: AtomicU64::new(1),
            bridge: RwLock::new(None),
        }
    }
}

impl DevToolsState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn next_uid(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn is_collapsed(&self, path: &[usize]) -> bool {
        self.collapsed_paths.read().contains(path)
    }

    pub fn toggle_collapsed(&self, path: &[usize]) {
        let mut set = self.collapsed_paths.write();
        if set.contains(path) {
            set.remove(path);
        } else {
            set.insert(path.to_vec());
        }
    }

    pub fn set_bridge(&self, bridge: crate::reactive::bridge::ReactiveBridge) {
        *self.bridge.write() = Some(bridge);
    }

    pub fn notify_bridge(&self) {
        if let Some(ref b) = *self.bridge.read() {
            b.notify();
        }
    }

    pub fn log(&self, level: LogLevel, msg: impl Into<String>) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        let entry = ConsoleEntry {
            id: self.next_uid(),
            level,
            time_str: chrono_like_time(),
            message: msg.into(),
        };
        let mut entries = self.console_entries.write();
        entries.push(entry);
        if entries.len() > 1000 {
            entries.remove(0);
        }
        drop(entries);
        self.notify_bridge();
    }

    pub fn record_http_request(
        &self,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: Option<String>,
    ) -> u64 {
        if !self.enabled.load(Ordering::Relaxed) {
            return 0;
        }
        let id = self.next_uid();
        let entry = NetworkEntry {
            id,
            entry_type: NetworkType::Fetch,
            method: method.to_uppercase(),
            url: url.to_string(),
            status: None,
            status_text: "Pending...".to_string(),
            start_time: Instant::now(),
            start_time_str: chrono_like_time(),
            duration_ms: None,
            request_headers: headers,
            request_body: body,
            response_headers: HashMap::new(),
            response_body: None,
            response_size: 0,
            ws_frames: Vec::new(),
            webrtc_stats: None,
        };
        self.network_entries.write().push(entry);
        self.notify_bridge();
        id
    }
    pub fn record_node_bounds(&self, path: &[usize], bounds: Bounds<Pixels>) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        self.node_bounds.write().insert(path.to_vec(), bounds);
    }

    pub fn record_http_response(
        &self,
        id: u64,
        status: u16,
        status_text: &str,
        headers: HashMap<String, String>,
        body: Option<String>,
        size: usize,
    ) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        let mut entries = self.network_entries.write();
        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
            entry.status = Some(status);
            entry.status_text = status_text.to_string();
            entry.duration_ms = Some(entry.start_time.elapsed().as_secs_f64() * 1000.0);
            entry.response_headers = headers;
            entry.response_body = body;
            entry.response_size = size;
        }
        drop(entries);
        self.notify_bridge();
    }

    pub fn record_ws_opened(&self, _ws_id: u64, url: &str) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        let id = self.next_uid();
        let entry = NetworkEntry {
            id,
            entry_type: NetworkType::WebSocket,
            method: "WS".to_string(),
            url: url.to_string(),
            status: Some(101),
            status_text: "Switching Protocols".to_string(),
            start_time: Instant::now(),
            start_time_str: chrono_like_time(),
            duration_ms: None,
            request_headers: HashMap::new(),
            request_body: None,
            response_headers: HashMap::new(),
            response_body: None,
            response_size: 0,
            ws_frames: Vec::new(),
            webrtc_stats: None,
        };
        self.network_entries.write().push(entry);
    }

    pub fn record_ws_message(&self, url: &str, is_outgoing: bool, payload: &str) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        let mut entries = self.network_entries.write();
        if let Some(entry) = entries
            .iter_mut()
            .rfind(|e| e.entry_type == NetworkType::WebSocket && e.url == url)
        {
            entry.ws_frames.push(WsFrameLog {
                is_outgoing,
                timestamp: chrono_like_time(),
                payload: payload.to_string(),
                size_bytes: payload.len(),
            });
            entry.response_size += payload.len();
        }
    }

    pub fn record_webrtc_peer(&self, peer_id: u64, state_info: &str) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        let mut entries = self.network_entries.write();
        if let Some(entry) = entries
            .iter_mut()
            .find(|e| e.entry_type == NetworkType::WebRtc && e.method == format!("PEER_{peer_id}"))
        {
            entry.webrtc_stats = Some(state_info.to_string());
        } else {
            let id = self.next_uid();
            entries.push(NetworkEntry {
                id,
                entry_type: NetworkType::WebRtc,
                method: format!("PEER_{peer_id}"),
                url: format!("webrtc://peer/{peer_id}"),
                status: Some(200),
                status_text: "Connected".to_string(),
                start_time: Instant::now(),
                start_time_str: chrono_like_time(),
                duration_ms: None,
                request_headers: HashMap::new(),
                request_body: None,
                response_headers: HashMap::new(),
                response_body: None,
                response_size: 0,
                ws_frames: Vec::new(),
                webrtc_stats: Some(state_info.to_string()),
            });
        }
    }
}

pub static ACTIVE_DEVTOOLS: parking_lot::RwLock<Option<Arc<DevToolsState>>> =
    parking_lot::RwLock::new(None);

pub fn set_active_devtools(state: Arc<DevToolsState>) {
    *ACTIVE_DEVTOOLS.write() = Some(state);
}

pub fn record_global_http_request(
    method: &str,
    url: &str,
    headers: HashMap<String, String>,
    body: Option<String>,
) -> Option<u64> {
    let dt = ACTIVE_DEVTOOLS.read();
    let dt = dt.as_ref()?;
    if !dt.enabled.load(Ordering::Relaxed) {
        return None;
    }
    Some(dt.record_http_request(method, url, headers, body))
}

pub fn record_global_http_response(
    id: u64,
    status: u16,
    status_text: &str,
    headers: HashMap<String, String>,
    body: Option<String>,
    size: usize,
) {
    if let Some(ref dt) = *ACTIVE_DEVTOOLS.read() {
        if dt.enabled.load(Ordering::Relaxed) {
            dt.record_http_response(id, status, status_text, headers, body, size);
        }
    }
}

pub fn record_global_ws_opened(ws_id: u64, url: &str) {
    if let Some(ref dt) = *ACTIVE_DEVTOOLS.read() {
        if dt.enabled.load(Ordering::Relaxed) {
            dt.record_ws_opened(ws_id, url);
        }
    }
}

pub fn record_global_ws_message(url: &str, is_outgoing: bool, payload: &str) {
    if let Some(ref dt) = *ACTIVE_DEVTOOLS.read() {
        if dt.enabled.load(Ordering::Relaxed) {
            dt.record_ws_message(url, is_outgoing, payload);
        }
    }
}

pub fn record_global_webrtc_peer(peer_id: u64, state_info: &str) {
    if let Some(ref dt) = *ACTIVE_DEVTOOLS.read() {
        if dt.enabled.load(Ordering::Relaxed) {
            dt.record_webrtc_peer(peer_id, state_info);
        }
    }
}
pub fn record_global_log(level: LogLevel, msg: impl Into<String>) {
    if let Some(ref dt) = *ACTIVE_DEVTOOLS.read() {
        if dt.enabled.load(Ordering::Relaxed) {
            dt.log(level, msg);
        }
    }
}

pub fn chrono_like_time() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = now.as_secs();
    let millis = now.subsec_millis();
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = (total_secs / 3600) % 24;
    format!("{hours:02}:{mins:02}:{secs:02}.{millis:03}")
}
