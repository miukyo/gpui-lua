# DSL & Component Styling

GPUI.lua provides a declarative, Tailwind-inspired layout and styling DSL. You construct components with chaining method syntax or property tables.

## Basic Layout (`ui.div`)

The fundamental layout container is `ui.div()`.

```lua
function App()
  return ui.div({
    w_full = true,
    h_full = true,
    bg = "#1e1e2e",
    items_center = true,
    justify_center = true,
    p = 24,
    children = {
      ui.text("Hello GPUI.lua!"):bold():size(24):color("#cdd6f4")
    }
  })
end
```

### Chaining Methods

You can chain style modifiers directly onto elements:

```lua
local box = ui.div()
  :w(200)
  :h(100)
  :bg("#313244")
  :rounded(8)
  :border(1)
  :border_color("#45475a")
  :p(12)
  :shadow_md()
```

### Supported Layout & Style Properties

| Property | Chaining Method | Type | Description |
| :--- | :--- | :--- | :--- |
| `w`, `h` | `:w(v)`, `:h(v)` | number | Width and height in pixels |
| `w_full`, `h_full` | `:w_full()`, `:h_full()` | boolean | Fill 100% of parent width/height |
| `p`, `px`, `py` | `:p(v)`, `:px(v)`, `:py(v)` | number | Padding in pixels |
| `m`, `mx`, `my` | `:m(v)`, `:mx(v)`, `:my(v)` | number | Margin in pixels |
| `bg` | `:bg(color)` | string | Background color (hex string) |
| `rounded` | `:rounded(r)` | number | Corner radius in pixels |
| `border` | `:border(w)` | number | Border width in pixels |
| `border_color` | `:border_color(c)` | string | Border color |
| `flex_row` | `:flex_row()` | boolean | Horizontal flexbox layout |
| `flex_col` | `:flex_col()` | boolean | Vertical flexbox layout |
| `items_center` | `:items_center()` | boolean | Align items along cross-axis |
| `justify_center` | `:justify_center()` | boolean | Justify content along main axis |
| `gap` | `:gap(g)` | number | Gap between flex children in pixels |
| `shadow_md` | `:shadow_md()` | boolean | Box shadow preset |

## Text Elements (`ui.text`)

Create text nodes using `ui.text(content)`:

```lua
ui.text("System Online")
  :size(16)
  :bold()
  :color("#a6e3a1")
```

## Custom Fonts & Font Inheritance

Load custom TrueType (`.ttf`) or OpenType (`.otf`) fonts from assets or the filesystem using `ui.load_font(path)`. The loaded font family name is automatically extracted and returned:

```lua
-- Load custom font from assets (returns family name, e.g. "Inter")
local font = ui.load_font("assets/fonts/Inter.ttf")

function App()
  return ui.div({
    -- Set font family on the root container
    font_family = font, -- or "Inter"
    children = {
      -- All children automatically inherit the parent's font family!
      ui.text("This text inherits Inter!"),
      ui.Button({ label = "Button text inherits Inter" }),
      ui.Card({
        title = "Card inherits Inter",
        children = {
          -- Can override font family on specific subtrees
          ui.text("Mono code"):font_family("Fira Code")
        }
      })
    }
  })
end
```
## Standard UI Components

The `ui` library includes pre-built components for common desktop patterns:

### Button (`ui.Button`)

Supports variants: `primary`, `secondary`, `danger`, `outline`, and `ghost`.

```lua
ui.Button({
  label = "Deploy Application",
  variant = "primary",
  on_click = function()
    print("Deployment started")
  end
})
```

### Card (`ui.Card`)

```lua
ui.Card({
  title = "System Diagnostics",
  subtitle = "Real-time GPU throughput and frame pacing",
  bg = "#313244",
  rounded = 12,
  p = 20,
  children = {
    ui.text("Status: Normal"):size(14):color("#cdd6f4")
  }
})
```

### Badge (`ui.Badge`)

```lua
ui.Badge({
  text = "v0.2.2-CE",
  color = "#b4befe",
  bg = "#45475a"
})
```

### Row and Stack (`ui.Row`, `ui.Stack`)

Layout shortcuts for horizontal and vertical stacking:

```lua
ui.Row({
  gap = 12,
  justify = "center",
  items = "center",
  children = {
    ui.Button({ label = "Cancel", variant = "ghost" }),
    ui.Button({ label = "Confirm", variant = "primary" })
  }
})
```

## Color Utilities

GPUI.lua provides built-in color manipulation helpers:

```lua
-- Format or normalize hex color strings
colors.hex("3b82f6")             -- returns "#3b82f6"

-- Format RGB integers (0-255)
colors.rgb(255, 100, 50)         -- returns "#ff6432"

-- Format RGBA with alpha float (0.0-1.0)
colors.rgba(255, 100, 50, 0.5)   -- returns "#ff64327f"

-- Add or modify alpha on existing hex colors
colors.alpha("#3b82f6", 0.2)     -- returns "#3b82f633"
```
