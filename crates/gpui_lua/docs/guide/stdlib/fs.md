# Filesystem (`fs`)

File and directory operations on background threads.

```lua
-- Write file
fs.write("config.json", '{"theme":"dark"}')

-- Read file
local data = fs.read("config.json")
print("Content:", data)

-- Check file existence
if fs.exists("config.json") then
    print("Found config.json")
end

-- Create directories recursively
fs.create_dir("output/logs")

-- List directory contents
local entries = fs.list_dir("./assets")
for _, entry in ipairs(entries) do
    print(entry.name, entry.is_dir, entry.size)
end

-- Delete file or directory
fs.remove("temp.txt")
```
