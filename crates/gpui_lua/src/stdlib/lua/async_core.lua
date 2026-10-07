-- Pure Luau Async / Await, Process, HTTP, FS, Dialog & Desktop Core
local async_core = {}

local function create_promise(executor)
    local p = {
        _state = "pending", -- "pending", "resolved", "rejected"
        _value = nil,
        _error = nil,
        _waiting = {}
    }

    local function resolve(val)
        if p._state ~= "pending" then return end
        p._state = "resolved"
        p._value = val
        for _, co in ipairs(p._waiting) do
            local ok, err = coroutine.resume(co, val)
            if not ok then
                print("[Async Error] " .. tostring(err))
            end
        end
        p._waiting = {}
    end

    local function reject(err)
        if p._state ~= "pending" then return end
        p._state = "rejected"
        p._error = err
        for _, co in ipairs(p._waiting) do
            local ok, resume_err = coroutine.resume(co, nil, err)
            if not ok then
                print("[Async Error] " .. tostring(resume_err))
            end
        end
        p._waiting = {}
    end

    function p:await()
        if p._state == "resolved" then
            return p._value
        elseif p._state == "rejected" then
            error(p._error)
        end

        local co = coroutine.running()
        if not co then
            error("Cannot :await() outside of an async.spawn coroutine task")
        end

        table.insert(p._waiting, co)
        local val, err = coroutine.yield()
        if p._state == "rejected" then
            error(err or p._error)
        end
        return val
    end

    if executor then
        executor(resolve, reject)
    end

    return p, resolve, reject
end

async_core.create_promise = create_promise

-- 1. Async Task Engine
local async = {
    spawn = function(fn)
        local co = coroutine.create(function()
            local ok, err = pcall(fn)
            if not ok then
                print("[Async Task Panic]: " .. tostring(err))
            end
        end)
        local ok, err = coroutine.resume(co)
        if not ok then
            print("[Async Spawn Error]: " .. tostring(err))
        end
        return co
    end,

    sleep = function(ms)
        local p, resolve = create_promise()
        if timer and timer.set_timeout then
            timer.set_timeout(ms, function() resolve(true) end)
        else
            resolve(true)
        end
        return p
    end
}

-- 2. Async HTTP Client
local http = {
    get = function(url, options)
        local p, resolve, reject = create_promise()
        __async_http_fetch({
            method = "GET",
            url = url,
            headers = options and options.headers,
            timeout = options and options.timeout
        }, function(err, resp)
            if err then
                reject(err)
            else
                resp.json = function()
                    return json.decode(resp.body)
                end
                resolve(resp)
            end
        end)
        return p
    end,

    post = function(url, body, options)
        local p, resolve, reject = create_promise()
        local body_str = (type(body) == "table") and json.encode(body) or tostring(body or "")
        __async_http_fetch({
            method = "POST",
            url = url,
            body = body_str,
            headers = options and options.headers,
            timeout = options and options.timeout
        }, function(err, resp)
            if err then
                reject(err)
            else
                resp.json = function()
                    return json.decode(resp.body)
                end
                resolve(resp)
            end
        end)
        return p
    end,

    fetch = function(options)
        local p, resolve, reject = create_promise()
        __async_http_fetch(options, function(err, resp)
            if err then
                reject(err)
            else
                resp.json = function()
                    return json.decode(resp.body)
                end
                resolve(resp)
            end
        end)
        return p
    end
}

-- 3. Async Filesystem
local fs_merged = {}
if fs then
    for k, v in pairs(fs) do
        fs_merged[k] = v
    end
end

fs_merged.read_file = function(path)
    local p, resolve, reject = create_promise()
    __async_fs_read(path, function(err, content)
        if err then reject(err) else resolve(content) end
    end)
    return p
end

fs_merged.write_file = function(path, data)
    local p, resolve, reject = create_promise()
    __async_fs_write(path, data, function(err, success)
        if err then reject(err) else resolve(success) end
    end)
    return p
end

fs_merged.list_dir = function(path)
    local p, resolve, reject = create_promise()
    __async_fs_list_dir(path, function(err, entries)
        if err then reject(err) else resolve(entries) end
    end)
    return p
end

fs_merged.remove = function(path)
    local p, resolve, reject = create_promise()
    __async_fs_remove(path, function(err, success)
        if err then reject(err) else resolve(success) end
    end)
    return p
end

fs_merged.create_dir = function(path)
    local p, resolve, reject = create_promise()
    __async_fs_create_dir(path, function(err, success)
        if err then reject(err) else resolve(success) end
    end)
    return p
end

if not fs_merged.exists then
    fs_merged.exists = function(path)
        local p, resolve = create_promise()
        __async_fs_exists(path, function(res)
            resolve(res)
        end)
        return p
    end
end

-- 4. Streaming Child Process Engine
local process = {
    spawn = function(cmd, args_tbl, options)
        local handle_id = __async_process_spawn(cmd, args_tbl or {}, options or {})
        local proc = {
            id = handle_id,
            _stdout_cbs = {},
            _stderr_cbs = {},
            _exit_promise = nil,
            _exit_resolve = nil
        }

        proc._exit_promise, proc._exit_resolve = create_promise()

        function proc:on_stdout(fn)
            table.insert(self._stdout_cbs, fn)
            return self
        end

        function proc:on_stderr(fn)
            table.insert(self._stderr_cbs, fn)
            return self
        end

        function proc:write_stdin(data)
            __async_process_stdin(self.id, tostring(data))
            return self
        end

        function proc:kill()
            __async_process_kill(self.id)
            return self
        end

        function proc:wait()
            return self._exit_promise
        end

        __register_process_handlers(handle_id, {
            on_stdout = function(chunk)
                for _, fn in ipairs(proc._stdout_cbs) do fn(chunk) end
            end,
            on_stderr = function(chunk)
                for _, fn in ipairs(proc._stderr_cbs) do fn(chunk) end
            end,
            on_exit = function(exit_data)
                proc._exit_resolve(exit_data)
            end
        })

        return proc
    end,

    exec = function(cmd, args_tbl)
        local p, resolve, reject = create_promise()
        local proc = process.spawn(cmd, args_tbl)
        local stdout_parts = {}
        local stderr_parts = {}

        proc:on_stdout(function(c) table.insert(stdout_parts, c) end)
        proc:on_stderr(function(c) table.insert(stderr_parts, c) end)

        async.spawn(function()
            local exit_info = proc:wait():await()
            resolve({
                code = exit_info.code,
                success = (exit_info.code == 0),
                stdout = table.concat(stdout_parts, ""),
                stderr = table.concat(stderr_parts, "")
            })
        end)

        return p
    end
}

-- 5. Native Desktop Dialogs
local dialog = {
    open_file = function(options)
        local p, resolve, reject = create_promise()
        __async_dialog_open_file(options or {}, function(err, result)
            if err then reject(err) else resolve(result) end
        end)
        return p
    end,

    save_file = function(options)
        local p, resolve, reject = create_promise()
        __async_dialog_save_file(options or {}, function(err, result)
            if err then reject(err) else resolve(result) end
        end)
        return p
    end,

    pick_folder = function(options)
        local p, resolve, reject = create_promise()
        __async_dialog_pick_folder(options or {}, function(err, result)
            if err then reject(err) else resolve(result) end
        end)
        return p
    end,

    message = function(msg, options)
        local p, resolve, reject = create_promise()
        __async_dialog_message(tostring(msg), options or {}, function(err, result)
            if err then reject(err) else resolve(result) end
        end)
        return p
    end
}

-- 6. Keyboard Hotkeys & Shortcuts
local hotkey = {
    bind = function(shortcut_str, callback)
        __hotkey_bind(shortcut_str, callback, false)
    end,

    bind_global = function(shortcut_str, callback)
        __hotkey_bind(shortcut_str, callback, true)
    end,

    unbind = function(shortcut_str)
        __hotkey_unbind(shortcut_str)
    end
}

-- 7. System Tray
local tray = {
    create = function(options)
        local id = __tray_create(options or {})
        return {
            id = id,
            set_title = function(self, t) __tray_set_title(self.id, t) end,
            set_tooltip = function(self, tt) __tray_set_tooltip(self.id, tt) end,
            destroy = function(self) __tray_destroy(self.id) end
        }
    end
}

async_core.async = async
async_core.http = http
async_core.fs = fs_merged
async_core.process = process
async_core.dialog = dialog
async_core.hotkey = hotkey
async_core.tray = tray

return async_core
