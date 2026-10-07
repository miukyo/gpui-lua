pub mod bridge;
pub mod store;

pub use bridge::ReactiveBridge;
pub use store::{ReactiveStore, StoredValue};

use mlua::{Lua, Result, Value};

pub fn register(lua: &Lua, store: ReactiveStore, bridge: ReactiveBridge) -> Result<()> {
    let store_get = store.clone();
    lua.globals().set(
        "__gpui_store_get",
        lua.create_function(move |lua, key: String| {
            if let Some(stored) = store_get.get(&key) {
                stored.to_lua_value(lua)
            } else {
                Ok(Value::Nil)
            }
        })?,
    )?;

    let store_set = store.clone();
    lua.globals().set(
        "__gpui_store_set",
        lua.create_function(move |_lua, (key, val): (String, Value)| {
            let stored = StoredValue::from_lua_value(&val);
            store_set.set(key, stored);
            Ok(())
        })?,
    )?;

    let store_has = store.clone();
    lua.globals().set(
        "__gpui_store_has",
        lua.create_function(move |_lua, key: String| Ok(store_has.has(&key)))?,
    )?;

    let bridge_notify = bridge.clone();
    lua.globals().set(
        "__gpui_notify",
        lua.create_function(move |_lua, ()| {
            bridge_notify.notify();
            Ok(())
        })?,
    )?;

    lua.load(r#"
        local current_subscriber = nil

        function signal(initial_val, opt_key)
            local key = opt_key
            if not key then
                local info = debug.getinfo(2, "Sl")
                if info and info.short_src and info.currentline then
                    key = info.short_src .. ":" .. tostring(info.currentline)
                else
                    key = "sig:" .. tostring({})
                end
            end

            if not __gpui_store_has(key) then
                __gpui_store_set(key, initial_val)
            end

            local subscribers = {}

            local function get(val)
                if val ~= nil then
                    -- Callable setter convenience
                    local old_val = __gpui_store_get(key)
                    if old_val ~= val then
                        __gpui_store_set(key, val)
                        for sub in pairs(subscribers) do
                            sub()
                        end
                        __gpui_notify()
                    end
                    return val
                end

                if current_subscriber then
                    subscribers[current_subscriber] = true
                end
                return __gpui_store_get(key)
            end

            local function set(new_val)
                local old_val = __gpui_store_get(key)
                if old_val ~= new_val then
                    __gpui_store_set(key, new_val)
                    for sub in pairs(subscribers) do
                        sub()
                    end
                    __gpui_notify()
                end
            end

            return get, set
        end

        function effect(fn)
            local function run()
                local prev = current_subscriber
                current_subscriber = run
                local ok, err = pcall(fn)
                current_subscriber = prev
                if not ok then
                    error(err)
                end
            end
            run()
            return run
        end

        function computed(fn)
            local dirty = true
            local cached_val = nil
            local subscribers = {}
            local run_comp

            run_comp = function()
                dirty = true
                for sub in pairs(subscribers) do
                    sub()
                end
                __gpui_notify()
            end

            local function get()
                if current_subscriber then
                    subscribers[current_subscriber] = true
                end
                if dirty then
                    local prev = current_subscriber
                    current_subscriber = run_comp
                    local ok, val = pcall(fn)
                    current_subscriber = prev
                    if not ok then
                        error(val)
                    end
                    cached_val = val
                    dirty = false
                end
                return cached_val
            end

            return get
        end

        function reactive(tbl)
            local orig = tbl or {}
            local subscribers = {}

            local proxy = {}
            local mt = {
                __index = function(_, k)
                    if current_subscriber then
                        if not subscribers[k] then subscribers[k] = {} end
                        subscribers[k][current_subscriber] = true
                    end
                    return orig[k]
                end,
                __newindex = function(_, k, v)
                    if orig[k] ~= v then
                        orig[k] = v
                        if subscribers[k] then
                            for sub in pairs(subscribers[k]) do
                                sub()
                            end
                        end
                        __gpui_notify()
                    end
                end,
                __pairs = function(_)
                    return pairs(orig)
                end,
            }
            return setmetatable(proxy, mt)
        end
    "#).exec()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_and_effect() -> Result<()> {
        let lua = Lua::new();
        let store = ReactiveStore::new();
        let bridge = ReactiveBridge::new();
        register(&lua, store.clone(), bridge.clone())?;

        lua.load(r#"
            local count, set_count = signal(0, "counter")
            local tracked_val = nil

            effect(function()
                tracked_val = count()
            end)

            assert(tracked_val == 0, "Initial effect failed")
            set_count(5)
            assert(tracked_val == 5, "Effect update failed")
        "#).exec()?;

        assert_eq!(store.get("counter"), Some(StoredValue::Integer(5)));
        Ok(())
    }

    #[test]
    fn test_computed_signal() -> Result<()> {
        let lua = Lua::new();
        let store = ReactiveStore::new();
        let bridge = ReactiveBridge::new();
        register(&lua, store, bridge)?;

        lua.load(r#"
            local count, set_count = signal(10, "c_val")
            local doubled = computed(function()
                return count() * 2
            end)

            assert(doubled() == 20, "Computed initial failed")
            set_count(25)
            assert(doubled() == 50, "Computed update failed")
        "#).exec()?;

        Ok(())
    }

    #[test]
    fn test_reactive_table() -> Result<()> {
        let lua = Lua::new();
        let store = ReactiveStore::new();
        let bridge = ReactiveBridge::new();
        register(&lua, store, bridge)?;

        lua.load(r#"
            local state = reactive({ name = "Alice", age = 30 })
            local observed = nil

            effect(function()
                observed = state.name .. " is " .. tostring(state.age)
            end)

            assert(observed == "Alice is 30", "Reactive initial failed")
            state.age = 31
            assert(observed == "Alice is 31", "Reactive update failed")
        "#).exec()?;

        Ok(())
    }

    #[test]
    fn test_state_preservation_across_reload() -> Result<()> {
        let store = ReactiveStore::new();
        let bridge = ReactiveBridge::new();

        // Run session 1
        {
            let lua1 = Lua::new();
            register(&lua1, store.clone(), bridge.clone())?;
            lua1.load(r#"
                local count, set_count = signal(0, "persist_key")
                set_count(42)
            "#).exec()?;
        }

        // Run session 2 (simulating reload with new Lua state or re-evaluated script)
        {
            let lua2 = Lua::new();
            register(&lua2, store.clone(), bridge.clone())?;
            let final_val: i64 = lua2.load(r#"
                local count, set_count = signal(0, "persist_key")
                return count()
            "#).eval()?;

            assert_eq!(final_val, 42, "Value should be preserved from ReactiveStore");
        }
        Ok(())
    }

    #[test]
    fn test_float_signal() -> Result<()> {
        let lua = Lua::new();
        let store = ReactiveStore::new();
        let bridge = ReactiveBridge::new();
        register(&lua, store, bridge)?;

        let res: f64 = lua.load(r#"
            local vol, set_vol = signal(1.0, "volume_test")
            set_vol(0.5)
            return vol() * 100
        "#).eval()?;

        assert_eq!(res, 50.0);
        Ok(())
    }
}
