# Builder Style Methods Reference

Fluent methods available on all elements returned by `ui.div`, `ui.text`, `ui.video`, `ui.img`, and `ui.svg`.

Every builder method returns `self` for fluent chaining.

---

## Sizing & Spacing

```lua
el:w(300)                  -- Fixed width in pixels
el:h(200)                  -- Fixed height in pixels
el:w_full()                -- Width 100%
el:h_full()                -- Height 100%
el:min_w(100):max_w(500)   -- Width constraints
el:min_h(50):max_h(400)    -- Height constraints
el:p(16)                   -- Padding all sides
el:px(12):py(8)            -- Horizontal / vertical padding
el:m(8)                    -- Margin all sides
el:mx(4):my(6)             -- Horizontal / vertical margin
el:gap(8)                  -- Flex item spacing
```

---

## Styling & Visuals

```lua
el:bg("#1e1e2e")           -- Background fill color
el:color("#cdd6f4")        -- Text color
el:rounded(8)              -- Corner radius
el:corner_smoothing(0.6)   -- Squircle smoothing (0.0 to 1.0)
el:rounded_smoothing_ios() -- iOS 0.6 smoothing
el:border(1)               -- Border width
el:border_color("#45475a") -- Border stroke color
el:border_dashed()         -- Dashed border stroke
el:opacity(0.8)            -- Element opacity (0.0 - 1.0)
el:shadow_md()             -- Hardware GPU drop shadow (_sm, _md, _lg)
el:backdrop_blur(24.0)     -- Frosted glass blur
```

---

## Overflow & Masks

```lua
el:overflow_hidden()       -- Clip children to bounds
el:overflow_scroll()       -- Enable scrollable viewport
el:overflow_fade_top(60.0) -- Attenuate top edge with gradient fade mask
el:overflow_fade_y(40.0)   -- Attenuate top and bottom edges
```

---

## Flexbox Alignment

```lua
el:flex_row()              -- Horizontal layout
el:flex_col()              -- Vertical layout
el:items_center()          -- Center along cross axis (_center, _start, _end)
el:justify_center()        -- Center along main axis (_center, _between, _start, _end)
```

---

## Interactivity & Events

```lua
el:cursor_pointer()        -- Hand pointer cursor on hover
el:stop_propagation()      -- Stop event bubbling up to parent

el:on_click(function()
    print("Clicked")
end)

el:on_mouse_down(function(e)
    print("Down at:", e.x, e.y, "button:", e.button)
end)

el:on_mouse_up(function(e)
    print("Up at:", e.x, e.y)
end)

el:on_mouse_move(function(e)
    print("Move at:", e.x, e.y)
end)

el:on_scroll_wheel(function(e)
    print("Scroll:", e.dx, e.dy)
end)

el:on_key_down(function(e)
    print("Key pressed:", e.key, "Ctrl:", e.ctrl, "Shift:", e.shift)
end)

el:on_drop(function(e)
    print("Dropped files:", e.paths)
end)
```

---

## Children & Text

```lua
el:child(ui.text("Child")) -- Append single child
el:children({ ... })       -- Append list of children
el:size(16)                -- Font size in pixels (text only)
el:bold()                  -- Bold weight (text only)
el:italic()                -- Italic style (text only)
el:line_height(24)         -- Line height in pixels (text only)
el:rows(4)                 -- Initial textarea rows count
el:max_rows(8)             -- Maximum textarea rows limit
```
