# Database (`db`)

Embedded SQLite database engine.

```lua
-- Open disk file or in-memory database
local conn = db.open("app.sqlite")
-- local conn = db.open_memory()

-- Execute DDL statement
conn:exec([[
    CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        email TEXT UNIQUE
    )
]])

-- Parameterized INSERT
conn:exec("INSERT INTO users (name, email) VALUES (?, ?)", { "Alice", "alice@example.com" })

-- Query all rows
local rows = conn:query("SELECT * FROM users ORDER BY id ASC")
for _, row in ipairs(rows) do
    print(row.id, row.name, row.email)
end

-- Query single row
local user = conn:query_row("SELECT * FROM users WHERE email = ?", { "alice@example.com" })
if user then
    print("User found:", user.name)
end

-- Close database connection
conn:close()
```
