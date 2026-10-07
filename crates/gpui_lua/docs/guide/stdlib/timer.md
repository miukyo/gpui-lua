# Timers (`timer`)

High-precision asynchronous timeouts and intervals.

```lua
-- One-shot timeout (delay in milliseconds)
local t1 = timer.timeout(1000, function()
    print("1000ms elapsed")
end)

-- Recurring interval
local count = 0
local t2 = timer.interval(500, function()
    count = count + 1
    print("Tick", count)
    if count >= 10 then
        timer.cancel(t2)
    end
end)

-- Cancel pending timer
timer.cancel(t1)
```
