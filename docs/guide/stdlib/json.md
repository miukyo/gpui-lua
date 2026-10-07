# JSON & Utilities (`json`)

Fast zero-copy JSON encoder and decoder.

```lua
-- Encode Lua table to JSON string
local payload = {
    user = "alice",
    age = 28,
    active = true,
    tags = { "ui", "gpu", "rust" }
}

local json_str = json.encode(payload)
print(json_str)

-- Decode JSON string back into Lua table
local decoded = json.decode(json_str)
print(decoded.user)    -- "alice"
print(decoded.tags[1]) -- "ui"
```
