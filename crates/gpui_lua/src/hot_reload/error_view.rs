use gpui::{
    AnyElement, FontWeight, IntoElement, ParentElement, SharedString, Styled,
    div, px, rgb, rgba,
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct HotReloadError {
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub traceback: Option<String>,
}

impl HotReloadError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            file: None,
            line: None,
            traceback: None,
        }
    }

    pub fn parse(raw_err: &str, trace: Option<&str>) -> Self {
        // e.g. "path/to/script.lua:15: attempt to index a nil value"
        let mut file = None;
        let mut line = None;

        if let Some((loc, _msg)) = raw_err.split_once(": ") {
            let parts: Vec<&str> = loc.split(':').collect();
            if parts.len() >= 2 {
                file = Some(parts[0].to_string());
                if let Ok(l) = parts[1].parse::<usize>() {
                    line = Some(l);
                }
            }
        }

        Self {
            message: raw_err.to_string(),
            file,
            line,
            traceback: trace.map(|s| s.to_string()),
        }
    }
}

pub fn render_error_view(err: &HotReloadError) -> AnyElement {
    let mut root = div()
        .w_full()
        .h_full()
        .bg(rgba(0x180808fa))
        .p(px(24.0))
        .flex()
        .flex_col()
        .gap(px(16.0))
        .items_start();

    // Header badge
    let header = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(12.0))
        .child(
            div()
                .bg(rgb(0xef4444))
                .text_color(rgb(0xffffff))
                .px(px(10.0))
                .py(px(4.0))
                .rounded(px(4.0))
                .font_weight(FontWeight::BOLD)
                .text_size(px(14.0))
                .child(SharedString::new_static("HOT RELOAD ERROR")),
        )
        .child(
            div()
                .text_color(rgb(0xfca5a5))
                .text_size(px(13.0))
                .child(SharedString::new_static(
                    "Edit and save file to resume execution",
                )),
        );

    root = root.child(header);

    // Location badge if file is known
    if let Some(file) = &err.file {
        let loc_text = if let Some(line) = err.line {
            format!("{file}:{line}")
        } else {
            file.clone()
        };

        root = root.child(
            div()
                .bg(rgba(0x450a0aff))
                .text_color(rgb(0xfecaca))
                .px(px(8.0))
                .py(px(4.0))
                .rounded(px(4.0))
                .text_size(px(12.0))
                .font_weight(FontWeight::BOLD)
                .child(SharedString::from(loc_text)),
        );
    }

    // Error description box
    let error_desc = div()
        .w_full()
        .bg(rgba(0x2d0606ff))
        .border(px(1.0))
        .border_color(rgb(0x7f1d1d))
        .rounded(px(6.0))
        .p(px(12.0))
        .child(
            div()
                .text_color(rgb(0xffffff))
                .font_weight(FontWeight::BOLD)
                .text_size(px(14.0))
                .child(SharedString::from(err.message.clone())),
        );

    root = root.child(error_desc);

    // Call stack traceback
    if let Some(tb) = &err.traceback {
        let traceback_box = div()
            .w_full()
            .bg(rgba(0x1c0404ff))
            .border(px(1.0))
            .border_color(rgb(0x450a0aff))
            .rounded(px(6.0))
            .p(px(12.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_color(rgb(0xf87171))
                    .text_size(px(12.0))
                    .font_weight(FontWeight::BOLD)
                    .child(SharedString::new_static("Lua Call Stack:")),
            )
            .child(
                div()
                    .text_color(rgb(0xd1d5db))
                    .text_size(px(12.0))
                    .child(SharedString::from(tb.clone())),
            );

        root = root.child(traceback_box);
    }

    root.into_any_element()
}
