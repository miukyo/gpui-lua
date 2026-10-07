# Chromium-like DevTools

GPUI.lua includes a built-in developer tools suite modeled after Chromium DevTools. It launches as an independent, detached native GPUI window with zero external browser or electron runtime dependencies.

---

## Opening DevTools

You can open and toggle the DevTools window using keyboard shortcuts or code:

### Keyboard Shortcuts

| Shortcut (Windows / Linux) | Shortcut (macOS) | Action |
| :--- | :--- | :--- |
| <kbd>F12</kbd> | <kbd>F12</kbd> | Toggle DevTools window |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>I</kbd> | <kbd>Cmd</kbd>+<kbd>Opt</kbd>+<kbd>I</kbd> | Toggle DevTools window |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> | <kbd>Cmd</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> | Toggle Inspect Cursor tool |

### Lua API

```lua
-- Open or toggle DevTools from script
ui.open_devtools()
devtools.toggle()
devtools.close()
```

### Rust API

```rust
use gpui_lua::LuaApp;

fn main() -> anyhow::Result<()> {
    LuaApp::new("main.lua")
        .open_devtools()
        .run()
}
```

---

## Elements Tab & Style Inspector

The **Elements** tab provides live inspection of the active UI component hierarchy:

- **DOM / Hierarchy Tree**:
  - Interactive tree of elements (`div`, `text`, `button`, `card`, `input`, `row`, etc.).
  - Shows `#id` selectors, text previews, and child element counts.
  - Selecting an element in the tree highlights it on the inspected application canvas.
- **Computed Styles Pane**:
  - Displays layout properties (`display`, `flex-direction`, `align-items`, `justify-content`, `gap`).
  - Dimensions (`width`, `height`, min/max constraints).
  - Spacing (`padding`, `margin`).
  - Typography (`font-family`, `font-size`, `color`).
  - Borders and backgrounds (`bg`, `border-width`, `border-color`, `border-radius`).
- **Interactive Box Model Diagram**:
  - Classic nested visual box diagram:
    - **Margin box** (orange `#f6b26b`, top/bottom/left/right)
    - **Border box** (yellow `#ffe599`)
    - **Padding box** (green `#b6d7a8`)
    - **Content box** (blue `#9fc5e8`, width × height)

---

## On-Canvas Box Model Overlay & Inspect Cursor

When an element is selected (or when the **Inspect Cursor** tool is active):

1. **Four Box-Model Rectangles**:
   - The application window draws colored translucent quads directly over the inspected element matching Chromium DevTools color conventions (Margin, Border, Padding, and Content).
2. **Floating Badge**:
   - Displays `<tag#id | W × H px>` directly above or below the element on screen.
3. **Inspect Cursor Tool (<kbd>Ctrl+Shift+C</kbd>)**:
   - Changes the cursor to crosshair.
   - Hovering over any element live-highlights its box model.
   - Clicking immediately selects the element in the DevTools tree and exits inspect mode.

---

## Network Tab (Fetch, WebSockets, WebRTC)

Monitors all application network activity in real time:

- **Filter Chips**: Filter by `All`, `Fetch/XHR`, `WebSocket`, or `WebRTC`.
- **HTTP / Fetch Requests**:
  - Method (`GET`, `POST`, `PUT`, `DELETE`), URL, status code (green `200`, red `404`/`500`), transfer size, duration (ms).
  - Request details drawer with General, Request Headers, Response Headers, and Response Body preview.
- **WebSocket Inspector**:
  - Connection lifecycle tracking (`101 Switching Protocols`, `Open`, `Closed`).
  - Live message stream with frame direction (`↑ SENT`, `↓ RECV`), timestamps, payload, and payload size.
- **WebRTC PeerConnections**:
  - Signaling state, ICE connection state, connection status, active local/remote audio and video tracks.

---

## Storage Tab (Database, LocalStorage, Signals)

Inspect and edit client storage and reactive state:

- **LocalStorage**:
  - Key-Value viewer for the persistent JSON store.
  - Live values preview.
- **SQLite Database Viewer**:
  - Lists open SQLite databases and database tables.
  - Interactive SQL Query Console: type queries (`SELECT * FROM my_table;`) and view results with column headers.
- **Reactive Signals**:
  - Live inspector for all signals managed by `signal(...)` in `ReactiveStore`.
  - Shows current values (`integer`, `number`, `string`, `boolean`, `json`).
  - Edit values live in DevTools and observe immediate UI re-renders!

---

## Console Tab & Lua REPL

- **Log Stream**:
  - Captures `print(...)`, `log.info`, `log.warn`, `log.error`, and runtime errors with timestamps and colored severity badges.
  - Filter by log level (`All`, `Errors`, `Warnings`, `Info`).
- **Live Interactive Lua REPL**:
  - Execute any Lua code directly inside the running application context.
  - Test functions, query signals (e.g. `return count()`), mutate state, or inspect objects.

---

## Performance & Telemetry Tab

- **Real-time Metrics**:
  - FPS counter and frame render duration (ms).
  - Active DOM node count.
  - LuaJIT memory usage in KB.
  - Window size and display scale factor.
