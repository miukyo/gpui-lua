use libloading::{Library, Symbol};
use mlua::{Lua, MultiValue, Result, Value};
use parking_lot::Mutex;
use std::sync::Arc;

pub struct LuaFfiLibrary(pub Arc<Mutex<Option<Library>>>);

impl mlua::UserData for LuaFfiLibrary {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        // lib:call(symbol_name, ...args) -> i64 or nil
        methods.add_method("call", |_lua, this, (symbol_name, args): (String, MultiValue)| {
            let guard = this.0.lock();
            let lib = guard
                .as_ref()
                .ok_or_else(|| mlua::Error::RuntimeError("FFI Library is closed".to_string()))?;

            unsafe {
                let sym_bytes = symbol_name.into_bytes();
                if args.is_empty() {
                    let func: Symbol<unsafe extern "C" fn() -> i64> = lib
                        .get(&sym_bytes)
                        .map_err(|e| mlua::Error::RuntimeError(format!("Symbol not found: {e}")))?;
                    let ret = func();
                    Ok(Value::Integer(ret))
                } else if args.len() == 1 {
                    let a1 = match args.get(0).unwrap() {
                        Value::Integer(i) => *i,
                        Value::Number(n) => *n as i64,
                        _ => 0,
                    };
                    let func: Symbol<unsafe extern "C" fn(i64) -> i64> = lib
                        .get(&sym_bytes)
                        .map_err(|e| mlua::Error::RuntimeError(format!("Symbol not found: {e}")))?;
                    let ret = func(a1);
                    Ok(Value::Integer(ret))
                } else if args.len() == 2 {
                    let a1 = match args.get(0).unwrap() {
                        Value::Integer(i) => *i,
                        Value::Number(n) => *n as i64,
                        _ => 0,
                    };
                    let a2 = match args.get(1).unwrap() {
                        Value::Integer(i) => *i,
                        Value::Number(n) => *n as i64,
                        _ => 0,
                    };
                    let func: Symbol<unsafe extern "C" fn(i64, i64) -> i64> = lib
                        .get(&sym_bytes)
                        .map_err(|e| mlua::Error::RuntimeError(format!("Symbol not found: {e}")))?;
                    let ret = func(a1, a2);
                    Ok(Value::Integer(ret))
                } else {
                    let a1 = match args.get(0).unwrap() {
                        Value::Integer(i) => *i,
                        Value::Number(n) => *n as i64,
                        _ => 0,
                    };
                    let a2 = match args.get(1).unwrap() {
                        Value::Integer(i) => *i,
                        Value::Number(n) => *n as i64,
                        _ => 0,
                    };
                    let a3 = match args.get(2).unwrap() {
                        Value::Integer(i) => *i,
                        Value::Number(n) => *n as i64,
                        _ => 0,
                    };
                    let func: Symbol<unsafe extern "C" fn(i64, i64, i64) -> i64> = lib
                        .get(&sym_bytes)
                        .map_err(|e| mlua::Error::RuntimeError(format!("Symbol not found: {e}")))?;
                    let ret = func(a1, a2, a3);
                    Ok(Value::Integer(ret))
                }
            }
        });

        // lib:close()
        methods.add_method("close", |_lua, this, ()| {
            let mut guard = this.0.lock();
            *guard = None;
            Ok(())
        });
    }
}

pub fn register(lua: &Lua) -> Result<()> {
    let ffi = lua.create_table();

    ffi.set("os", std::env::consts::OS)?;
    ffi.set("arch", std::env::consts::ARCH)?;

    // ffi.load(path) -> library handle
    ffi.set(
        "load",
        lua.create_function(|_lua, path: String| {
            unsafe {
                let lib = Library::new(&path)
                    .map_err(|e| mlua::Error::RuntimeError(format!("Failed to load library '{path}': {e}")))?;
                Ok(LuaFfiLibrary(Arc::new(Mutex::new(Some(lib)))))
            }
        })?,
    )?;

    lua.globals().set("ffi", ffi.clone())?;

    if let Ok(pkg) = lua.globals().get::<mlua::Table>("package") {
        if let Ok(loaded) = pkg.get::<mlua::Table>("loaded") {
            let _ = loaded.set("ffi", ffi);
        }
    }

    Ok(())
}
