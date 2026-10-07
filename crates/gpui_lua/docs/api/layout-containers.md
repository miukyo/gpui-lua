# Layout Containers Reference

Layout shorthand primitives for assembling interfaces.

---

## `ui.row(props)`
Horizontal flex container (`flex_row = true`, `items_center = true`).

```lua
ui.row({
    gap = 8,
    align = "center",
    justify = "between",
    w_full = true,
    children = {
        ui.text("Left"),
        ui.text("Right")
    }
})
```

---

## `ui.column(props)`
Vertical flex container (`flex_col = true`).

```lua
ui.column({
    gap = 12,
    p = 16,
    w_full = true,
    children = {
        ui.text("Header"),
        ui.text("Body")
    }
})
```

---

## `ui.stack(props)`
Overlay container where children stack on top of each other along the Z-axis.

```lua
ui.stack({
    w_full = true,
    h = 200,
    children = {
        ui.div({ bg = "#000000", w_full = true, h_full = true }),
        ui.text("Centered Text"):color("#ffffff")
    }
})
```
