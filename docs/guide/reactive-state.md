# Reactive State Management

GPUI.lua adopts a fine-grained reactivity model inspired by SolidJS and Preact Signals. State transitions trigger layout updates automatically.

## The `signal` Primitive

Create reactive signals using the global `signal(initial_value, [key])`:

```lua
local count, set_count = signal(0, "counter")

-- Read the signal value
print("Current count:", count())

-- Update the signal value
set_count(count() + 1)
```

### Hot-Reload State Preservation

The second parameter to `signal()` is an optional key. When a key is provided, the runtime registers the value in an underlying persistent store:

```lua
local count, set_count = signal(0, "user_counter")
```

When you edit `main.lua` during `gpui-lua dev`:
1. The script file is re-evaluated by the watcher.
2. The runtime preserves the current value associated with `"user_counter"`.
3. The UI tree re-renders with the existing state intact instead of resetting to `0`!

## Complex Reactive State

Signals can hold any Lua primitive, table, or array:

```lua
local user_profile, set_profile = signal({
  username = "alice",
  role = "developer",
  active = true
}, "profile_state")

function App()
  local user = user_profile()

  return ui.Card({
    title = user.username,
    subtitle = "Role: " .. user.role,
    children = {
      ui.Button({
        label = user.active and "Deactivate" or "Activate",
        variant = user.active and "danger" or "primary",
        on_click = function()
          local updated = {
            username = user.username,
            role = user.role,
            active = not user.active
          }
          set_profile(updated)
        end
      })
    }
  })
end
```

## Reactive Flow Guarantees

- **Batching**: Multiple signal updates within a single click callback batch into a single layout and render frame.
- **Zero Polling**: Render cycles execute only when an action or event triggers a signal mutation.
- **Rust-Thread Safety**: State stores sync across GPUI foreground and worker thread boundaries.
