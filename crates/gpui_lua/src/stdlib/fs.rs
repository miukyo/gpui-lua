use mlua::{Lua, Result, Value};
use std::path::Path;

pub fn register(lua: &Lua) -> Result<()> {
    let fs = lua.create_table();

    // fs.read(path) -> string, err
    fs.set(
        "read",
        lua.create_function(|lua, path: String| match std::fs::read_to_string(&path) {
            Ok(content) => Ok((Value::String(lua.create_string(&content)), Value::Nil)),
            Err(e) => Ok((
                Value::Nil,
                Value::String(lua.create_string(&e.to_string())),
            )),
        })?,
    )?;

    // fs.write(path, data) -> bool, err
    fs.set(
        "write",
        lua.create_function(|lua, (path, data): (String, Value)| {
            let bytes = match data {
                Value::String(s) => s.as_bytes().to_vec(),
                Value::Integer(i) => i.to_string().into_bytes(),
                Value::Number(n) => n.to_string().into_bytes(),
                _ => Vec::new(),
            };

            if let Some(parent) = Path::new(&path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            match std::fs::write(&path, bytes) {
                Ok(()) => Ok((Value::Boolean(true), Value::Nil)),
                Err(e) => Ok((
                    Value::Boolean(false),
                    Value::String(lua.create_string(&e.to_string())),
                )),
            }
        })?,
    )?;

    // fs.exists(path) -> bool
    fs.set(
        "exists",
        lua.create_function(|_lua, path: String| Ok(Path::new(&path).exists()))?,
    )?;

    lua.globals().set("fs", fs)?;
    Ok(())
}
