# Standard Library Modules

GPUI.lua includes built-in standard library modules bridging system APIs without requiring native C FFI boilerplate.

## Timer (`timer`)

Asynchronous timeouts and intervals integrated directly with the GPUI event loop.

```lua
-- Single shot delayed execution (in milliseconds)
local timer_id = timer.set_timeout(function()
  print("Executed after 1000ms delay")
end, 1000)

-- Recurring interval execution
local interval_id = timer.set_interval(function()
  print("Tick every 500ms")
end, 500)

-- Cancel timers
timer.clear_timeout(timer_id)
timer.clear_interval(interval_id)
```

## Filesystem (`fs`)

Synchronous and asynchronous file I/O operations.

```lua
-- Read entire file as string
local content = fs.read("data.json")

-- Write string or bytes to file
fs.write("config.txt", "key=value\n")

-- Check existence
if fs.exists("data.json") then
  print("File exists")
end

-- Read directory entries
local entries = fs.read_dir("./assets")
for _, entry in ipairs(entries) do
  print("Found file:", entry.name, entry.is_dir)
end
```

## JSON (`json`)

High-performance JSON serialization and deserialization using `serde_json`.

```lua
local raw_json = '{"name": "GPUI Application", "version": "0.1.0"}'

-- Parse JSON string to Lua table
local data = json.decode(raw_json)
print("Parsed name:", data.name)

-- Encode Lua table to JSON string
local encoded = json.encode({ status = "ready", code = 200 })
print("Encoded JSON:", encoded)
```

## Operating System (`os`)

Cross-platform system information and process utilities.

```lua
-- Platform name: "windows", "macos", or "linux"
print("Current OS:", os.platform())

-- Get environment variable
local home = os.getenv("HOME") or os.getenv("USERPROFILE")
print("User Home:", home)
```
