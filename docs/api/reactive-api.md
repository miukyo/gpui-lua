# Reactive State API Reference

Fine-grained reactive state primitives for `gpui.lua`.

---

## `signal(initial_val, [key])`

Creates a reactive state pair `get, set`. When `key` is provided, state survives code hot-reloads transparently.

```lua
local count, set_count = signal(0, "app_counter")

-- Read value
local current = count()

-- Write value (triggers UI re-render)
set_count(count() + 1)
```

---

## `computed(fn)`

Creates a read-only derived value. Automatically re-runs only when referenced signals change.

```lua
local width, set_width = signal(100, "w")
local height, set_height = signal(50, "h")

local area = computed(function()
    return width() * height()
end)

print("Area:", area()) -- 5000
```

---

## `effect(fn)`

Executes a side-effect function whenever any read signal changes value.

```lua
local query, set_query = signal("", "search")

effect(function()
    print("Search query updated:", query())
end)
```
