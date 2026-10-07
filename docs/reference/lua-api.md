# Lua API Reference

Comprehensive reference for the GPUI.lua UI DSL, reactive primitives, and stdlib.

## Globals

### `signal(initial_value, [key])`
Creates a reactive state signal.
- **Parameters**:
  - `initial_value: any`: Starting value (number, string, table, boolean).
  - `key: string?`: Optional persistent key for hot-reload state preservation.
- **Returns**: `(getter: function, setter: function)`
  - `getter()`: Returns the current signal value.
  - `setter(new_value)`: Sets the signal value and schedules a UI re-render.

---

## UI Library (`ui`)

### `ui.div([props])`
Creates a generic layout container element.
- **Props**:
  - `w`: Width in pixels.
  - `h`: Height in pixels.
  - `w_full`: boolean, expand to 100% width.
  - `h_full`: boolean, expand to 100% height.
  - `bg`: Hex background color string.
  - `text_color`: Hex text color string.
  - `p`, `px`, `py`: Padding in pixels.
  - `m`, `mx`, `my`: Margin in pixels.
  - `rounded`: Corner radius in pixels.
  - `border`: Border width in pixels.
  - `border_color`: Border color string.
  - `flex_row`: boolean, layout children horizontally.
  - `flex_col`: boolean, layout children vertically.
  - `items_center`: boolean, center items on cross-axis.
  - `justify_center`: boolean, center items on main axis.
  - `gap`: Gap between children in pixels.
  - `shadow_md`: boolean, apply medium drop shadow.
  - `on_click`: Click event callback function.
  - `children`: Table array of child elements.
- **Methods**:
  - `:child(elem)`: Append a child element.
  - `:children(list)`: Append a table of child elements.
  - `:on_click(fn)`: Attach a click listener.
  - `:w(n)`, `:h(n)`, `:w_full()`, `:h_full()`
  - `:p(n)`, `:px(n)`, `:py(n)`, `:m(n)`, `:mx(n)`, `:my(n)`
  - `:bg(color)`, `:color(color)`
  - `:rounded(r)`, `:border(w)`, `:border_color(c)`
  - `:flex_row()`, `:flex_col()`, `:items_center()`, `:justify_center()`, `:gap(g)`
  - `:shadow_md()`

### `ui.text(content)`
Creates a text node.
- **Methods**:
  - `:size(n)`: Font size in pixels.
  - `:bold()`: Bold font weight.
  - `:color(hex)`: Text color.

### `ui.Button(props)`
Pre-styled button component.
- **Props**:
  - `label`: Button text string.
  - `variant`: `"primary"` | `"secondary"` | `"danger"` | `"outline"` | `"ghost"`.
  - `on_click`: Callback function.
  - `disabled`: boolean.

### `ui.Card(props)`
Surface card container.
- **Props**:
  - `title`: Optional header title string.
  - `subtitle`: Optional header subtitle string.
  - `bg`: Background color.
  - `rounded`: Corner radius (default 8).
  - `p`: Padding (default 16).
  - `children`: Child elements array.

### `ui.Badge(props)`
Pill badge indicator.
- **Props**:
  - `text`: Badge label.
  - `color`: Text color.
  - `bg`: Pill background color.

### `ui.Row(props)`
Horizontal flex layout helper.
- **Props**:
  - `gap`: Gap in pixels.
  - `justify`: `"center"` | `"start"` | `"end"`.
  - `items`: `"center"` | `"start"` | `"end"`.
  - `children`: Child elements array.

### `ui.Stack(props)`
Vertical flex layout helper.
- **Props**:
  - `gap`: Gap in pixels.
  - `items`: Align items mode.
  - `children`: Child elements array.
---
---

### `ui.load_font(path)` / `ui.add_font(path)`
Loads a TrueType (`.ttf`) or OpenType (`.otf`) font from assets or filesystem.
- **Parameters**: `path: string` — File path or embedded asset name.
- **Returns**: `font_family: string` — The font family name extracted from the font metadata.
- **Inheritance**: Setting `font_family = "FamilyName"` on any container automatically cascades down to all child elements (`ui.text`, `ui.Button`, `ui.input`, `ui.Card`, etc.) unless explicitly overridden.

```lua
local inter = ui.load_font("assets/fonts/Inter-Regular.ttf")

function App()
  return ui.div({
    font_family = inter, -- all child elements inherit this font!
    children = {
      ui.text("Inherits Inter"),
      ui.Button({ label = "Button inherits Inter" })
    }
  })
end
```

---
### Custom Elements (`ui.<custom_element>`)

Any native GPUI component registered from Rust via `app.register_element("name", ...)` is automatically exposed as a constructor on the `ui` module:

```lua
-- Table props syntax
local el = ui.my_widget({
  theme = "dark",
  count = 42,
  children = { ui.text("Child content") }
})

-- Chaining syntax
local el = ui.my_widget()
  :prop("theme", "dark")
  :prop("count", 42)
  :child(ui.text("Child content"))
```

---

## Colors (`colors`)

### Color Helpers
- `colors.hex(code)`: Normalizes hex code with leading `#`.
- `colors.rgb(r, g, b)`: Returns `#rrggbb` hex string.
- `colors.rgba(r, g, b, a)`: Returns `#rrggbbaa` hex string.
- `colors.alpha(hex, a)`: Applies alpha `0.0..1.0` to hex color.
