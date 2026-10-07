# JSON RPC Functions

GPUI.lua provides high-level JSON RPC bindings that allow exchanging complex structured data between Rust and Lua without manually constructing low-level Lua types.

## Registering JSON Functions in Rust

Use `register_json_fn` to expose endpoints that receive and return arbitrary JSON:

```rust
use gpui_lua::LuaApp;
use serde_json::json;

fn main() -> anyhow::Result<()> {
    let mut app = LuaApp::new("ui/app.lua")?;

    app.register_json_fn("query_database", |params: serde_json::Value| {
        let table_name = params["table"].as_str().unwrap_or("users");
        let limit = params["limit"].as_u64().unwrap_or(10);

        Ok(json!({
            "status": "ok",
            "table": table_name,
            "rows": [
                { "id": 1, "name": "Alice" },
                { "id": 2, "name": "Bob" }
            ],
            "total": 2
        }))
    })?;

    app.run()
}
```

## Invoking from Lua

JSON arguments are automatically converted from Lua tables to serde values, and the returned JSON is converted back into a native Lua table:

```lua
local query_result, set_query_result = signal(nil, "db_data")

local function run_query()
  local response = backend.query_database({
    table = "users",
    limit = 25
  })

  if response.status == "ok" then
    print("Fetched rows count:", #response.rows)
    for _, row in ipairs(response.rows) do
      print(string.format("User #%d: %s", row.id, row.name))
    end
    set_query_result(response.rows)
  end
end
```

## Structured Error Handling

If the Rust function returns `anyhow::Error`, the Lua invocation raises a catchable error or passes an error table with code and message:

```rust
app.register_json_fn("dangerous_op", |params| {
    if params["authorized"] != true {
        anyhow::bail!("Unauthorized access: token missing or expired");
    }
    Ok(json!({ "success": true }))
})?;
```

In Lua:

```lua
local ok, result = pcall(function()
  return backend.dangerous_op({ authorized = false })
end)

if not ok then
  print("RPC failed gracefully:", result)
end
```
