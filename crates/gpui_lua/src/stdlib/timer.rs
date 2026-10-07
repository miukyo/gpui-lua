use crate::reactive::bridge::ReactiveBridge;
use mlua::{Function, Lua, Result};
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

static NEXT_TIMER_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct TimerEngine {
    bridge: ReactiveBridge,
    lua_ref: Option<Arc<Mutex<Lua>>>,
    active_timers: Arc<RwLock<HashMap<u64, Arc<AtomicBool>>>>,
}

impl TimerEngine {
    pub fn new(bridge: ReactiveBridge) -> Self {
        Self {
            bridge,
            lua_ref: None,
            active_timers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_lua(mut self, lua: Arc<Mutex<Lua>>) -> Self {
        self.lua_ref = Some(lua);
        self
    }

    pub fn clear(&self, id: u64) {
        if let Some(flag) = self.active_timers.write().remove(&id) {
            flag.store(false, Ordering::SeqCst);
        }
    }
}

pub fn register(lua: &Lua, engine: TimerEngine) -> Result<()> {
    let timer = lua.create_table();

    // timer.set_timeout(ms, callback) -> id
    let timeout_engine = engine.clone();
    timer.set(
        "set_timeout",
        lua.create_function(move |lua, (ms, callback): (u64, Function)| {
            let id = NEXT_TIMER_ID.fetch_add(1, Ordering::SeqCst);
            let active_flag = Arc::new(AtomicBool::new(true));
            timeout_engine
                .active_timers
                .write()
                .insert(id, active_flag.clone());

            let cb_key = lua.create_registry_value(callback)?;
            let lua_holder = timeout_engine.lua_ref.clone();
            let bridge = timeout_engine.bridge.clone();
            let timers_map = timeout_engine.active_timers.clone();

            crate::tokio_runtime().spawn(async move {
                tokio::time::sleep(Duration::from_millis(ms)).await;
                if active_flag.load(Ordering::SeqCst) {
                    if let Some(lua_arc) = lua_holder {
                        let lua = lua_arc.lock();
                        if let Ok(func) = lua.registry_value::<Function>(&cb_key) {
                            let _ = func.call::<()>(());
                        }
                    }
                    bridge.notify();
                }
                timers_map.write().remove(&id);
            });

            Ok(id)
        })?,
    )?;

    // timer.set_interval(ms, callback) -> id
    let interval_engine = engine.clone();
    timer.set(
        "set_interval",
        lua.create_function(move |lua, (ms, callback): (u64, Function)| {
            let id = NEXT_TIMER_ID.fetch_add(1, Ordering::SeqCst);
            let active_flag = Arc::new(AtomicBool::new(true));
            interval_engine
                .active_timers
                .write()
                .insert(id, active_flag.clone());

            let cb_key = lua.create_registry_value(callback)?;
            let lua_holder = interval_engine.lua_ref.clone();
            let bridge = interval_engine.bridge.clone();
            let flag = active_flag.clone();

            crate::tokio_runtime().spawn(async move {
                while flag.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_millis(ms)).await;
                    if !flag.load(Ordering::SeqCst) {
                        break;
                    }
                    if let Some(lua_arc) = lua_holder.as_ref() {
                        let lua = lua_arc.lock();
                        if let Ok(func) = lua.registry_value::<Function>(&cb_key) {
                            let _ = func.call::<()>(());
                        }
                    }
                    bridge.notify();
                }
            });

            Ok(id)
        })?,
    )?;

    // timer.clear(id)
    let clear_engine = engine.clone();
    timer.set(
        "clear",
        lua.create_function(move |_lua, id: u64| {
            clear_engine.clear(id);
            Ok(())
        })?,
    )?;

    lua.globals().set("timer", timer)?;
    Ok(())
}
