# HTTP & Networking (`http`, `net`)

Asynchronous HTTP requests, WebSockets, and Server-Sent Events.

---

## HTTP Client (`http`)

```lua
-- GET request
http.get("https://httpbin.org/get", function(err, resp)
    if not err and resp.status == 200 then
        print("Response:", resp.body)
    end
end)

-- POST request with JSON
local headers = { ["Content-Type"] = "application/json" }
local payload = '{"username":"developer"}'

http.post("https://httpbin.org/post", payload, headers, function(err, resp)
    if not err then
        print("Status:", resp.status)
        print("Headers:", resp.headers)
    end
end)
```

---

## Full-Duplex WebSockets (`net.websocket`)

```lua
local ws = net.websocket("wss://echo.websocket.events")

ws:on_open(function()
    print("WebSocket connected!")
    ws:send("Hello from GPUI.lua!")
end)

ws:on_message(function(msg)
    print("Received:", msg)
end)

ws:on_close(function(code, reason)
    print("Closed:", code, reason)
end)

ws:on_error(function(err)
    print("Error:", err)
end)
```

---

## Server-Sent Events (`net.sse`)

```lua
local sse = net.sse("https://example.com/events")

sse:on_message(function(data)
    print("SSE message:", data)
end)

sse:on_event("ping", function(data)
    print("Ping event:", data)
end)

-- Close stream
sse:close()
```
