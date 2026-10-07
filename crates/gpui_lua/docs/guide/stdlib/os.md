# Operating System (`os`)

System diagnostics, clipboard, process execution, and paths.

```lua
-- Host platform identification ("windows", "macos", "linux")
local platform = os.platform()

-- Standard system directories
local home = os.home_dir()
local appdata = os.appdata_dir()

-- Window metrics ({ width, height, scale_factor })
local win = os.window_info()

-- Clipboard read / write
os.clipboard_set("Copied to system clipboard")
local text = os.clipboard_get()

-- Spawn external process
local res = os.exec("git", { "status", "--short" })
print("Exit code:", res.exit_code)
print("Output:", res.stdout)
```
