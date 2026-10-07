<img src="assets/readme/banner.png" alt="GPUI Community Edition banner" width="100%">

<p align="center">
  <a href="https://gpui-ce.github.io">Website</a> ​ · ​ <a href="crates/gpui/examples/learn">Examples</a> ​ · ​ <a href="https://discord.gg/ENGHGjrYEn">Discord</a>
</p>

# GPUI-CE — fork for [gpui.lua](crates/gpui_lua)

This repository is **not** the upstream GPUI-CE community edition. It is a personal fork of it,
maintained as the engine behind [gpui.lua](crates/gpui_lua) — a Lua UI framework built on GPUI.

Its goal is to give Lua apps the parts of GPUI that upstream leaves out: real media playback,
working translucency, and native-looking windows.

**All gpui.lua documentation lives in [`crates/gpui_lua`](crates/gpui_lua) — start there.**

## Why this fork exists

| Change | Why |
| --- | --- |
| Video & audio elements via FFmpeg | `video` / `audio` elements with hardware-accelerated decoding, so a Lua app can play media without shelling out to a browser engine |
| Backdrop blur now works with overflow fade | The blur effect was silently dropped whenever a container also faded its overflow; the two now compose correctly |
| macOS Metal renderer replaced by WGPU | macOS now renders through the same WGPU path as Linux and the web, instead of a separate Metal implementation. Windows still renders with DirectX |
| CSD for custom titlebars | Client-side decorations on Windows, so `titlebar` styled windows can actually look like they belong to the app instead of the OS |

Alongside these it carries the usual stream of upstream GPUI-CE fixes: rendering and path-drawing
stability, window background handling, accessibility improvements, animation work, and more.

The Rust API stays close to GPUI-CE upstream — the divergence is in features and platform
internals, not in everyday `div()` usage.

## Overview:

- Web-inspired Styling & Layout

  Build views with familiar elements, flex layouts, and Tailwind-style methods:

  ```rust
  div()
      .id("some_id_123")
      .flex()
      .items_center()
      .gap_2()
      .rounded_lg()
      .rounded_smoothing(0.8)
      .bg(rgba(0xffffff30))
      .backdrop_blur(px(12.0))
      .transitions(|transitions| transitions.bg(millis(200)))
      .hover(|style| style.bg(rgba(0xffffff60)))
      .child("Hello, GPUI")
  ```

  [Layout example](crates/gpui/examples/learn/layout.rs) ​ · ​ [Styling example](crates/gpui/examples/learn/styling.rs)

- State and events

  Use `Entity<T>` to access view and shared application state. Observe changes and call `cx.notify()` when state changes to notify observers and update the view. For a view with a `count` field:

  ```rust
  div()
      .id("counter")
      .child(format!("Count: {}", self.count))
      .on_click(cx.listener(|this, _event, _window, cx| {
          this.count += 1;
          cx.notify();
      }))
  ```

  [Interaction example](crates/gpui/examples/learn/interactive_elements.rs)

- Actions and keybinds

  Define typed actions and bind them to keyboard shortcuts. Call `register_keybinds` during app setup to bind Space and Backspace in the focused counter:

  ```rust
  use gpui::{App, KeyBinding, actions};

  actions!(keybinds_example, [Increment, Reset]);

  fn register_keybinds(cx: &mut App) {
      cx.bind_keys([
          KeyBinding::new("space", Increment, Some("Counter")),
          KeyBinding::new("backspace", Reset, Some("Counter")),
      ]);
  }
  ```

  Connect the view's focus handle and action handlers:

  ```rust
  div()
      .key_context("Counter")
      .track_focus(&self.focus_handle)
      .on_action(cx.listener(Self::increment))
      .on_action(cx.listener(Self::reset))
      .child(format!("Count: {}", self.count))
  ```

  [Keybind example](crates/gpui/examples/learn/actions_and_keybinds.rs)

- Virtualized lists

  Use `uniform_list` for large collections of equal-height rows. GPUI requests the item ranges needed for the visible area as you scroll.

  ```rust
  uniform_list("items", 10_000, |range, _window, _cx| {
      range
          .map(|index| {
              div()
                  .h(px(24.0))
                  .child(format!("Item {index}"))
          })
          .collect()
  })
  .h(px(300.0))
  ```

  [List example](crates/gpui/examples/learn/uniform_list.rs)

- Custom drawing

  Use `canvas` to paint directly within a view. Implement `Element` when you need control over layout and rendering, such as for a code editor or custom widget.

  ```rust
  canvas(
      |_bounds, _window, _cx| {},
      |bounds, _state, window, _cx| {
          window.paint_quad(fill(bounds, rgb(0x5078f0)));
      },
  )
  .size(px(80.0))
  ```

  [Drawing example](crates/gpui/examples/learn/custom_drawing.rs)

## Setup
View the [setup guide](SETUP.md) for installation instructions.

## FAQ
- Q: Where do I start?
  A: [`crates/gpui_lua`](crates/gpui_lua). It has the getting-started guide, the reactive state and DSL
  guides, CSD/window docs, media and WebRTC API references, and the CLI reference for `gpui.lua run|dev|build`.

- Q: I'm writing a Rust GUI app, not a Lua app.
  A: Use it anyway — the Rust crates here are ordinary GPUI-CE crates and the examples under
  `crates/gpui/examples/learn` still apply.

- Q: Is this the official GPUI-CE community edition?
  A: No. Please take general questions, bug reports, and AI-policy questions to the upstream
  [GPUI-CE project](https://github.com/gpui-ce/gpui-ce).
