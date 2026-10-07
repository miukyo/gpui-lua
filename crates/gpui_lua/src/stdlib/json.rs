use crate::net::json_to_lua_value;
use mlua::{Lua, Result, Value};

pub fn lua_value_to_json(val: Value) -> Result<serde_json::Value> {
    match val {
        Value::Nil => Ok(serde_json::Value::Null),
        Value::Boolean(b) => Ok(serde_json::Value::Bool(b)),
        Value::Integer(i) => Ok(serde_json::Value::Number(i.into())),
        Value::Number(n) => {
            if let Some(num) = serde_json::Number::from_f64(n) {
                Ok(serde_json::Value::Number(num))
            } else {
                Ok(serde_json::Value::Null)
            }
        }
        Value::String(s) => Ok(serde_json::Value::String(s.to_str()?.to_string())),
        Value::Table(t) => {
            // Check if table is array (consecutive integer keys starting from 1)
            let mut is_array = true;
            let mut count = 0;
            for pair in t.pairs::<Value, Value>() {
                count += 1;
                let (k, _) = pair?;
                match k {
                    Value::Integer(i) if i == count => {}
                    _ => {
                        is_array = false;
                        break;
                    }
                }
            }

            if is_array && count > 0 {
                let mut arr = Vec::new();
                for i in 1..=count {
                    let v: Value = t.get(i)?;
                    arr.push(lua_value_to_json(v)?);
                }
                Ok(serde_json::Value::Array(arr))
            } else {
                let mut map = serde_json::Map::new();
                for pair in t.pairs::<Value, Value>() {
                    let (k, v) = pair?;
                    let key_str = match k {
                        Value::String(s) => s.to_str()?.to_string(),
                        Value::Integer(i) => i.to_string(),
                        Value::Number(n) => n.to_string(),
                        _ => continue,
                    };
                    map.insert(key_str, lua_value_to_json(v)?);
                }
                Ok(serde_json::Value::Object(map))
            }
        }
        _ => Ok(serde_json::Value::Null),
    }
}

pub fn register(lua: &Lua) -> Result<()> {
    let json = lua.create_table();

    // json.encode(val) -> string
    json.set(
        "encode",
        lua.create_function(|_lua, val: Value| {
            let json_val = lua_value_to_json(val)?;
            serde_json::to_string(&json_val)
                .map_err(|e| mlua::Error::RuntimeError(format!("JSON encode error: {e}")))
        })?,
    )?;

    // json.decode(str) -> val
    json.set(
        "decode",
        lua.create_function(|lua, s: Option<String>| {
            let json_str = s.unwrap_or_default();
            if json_str.trim().is_empty() {
                return Ok(Value::Nil);
            }
            let json_val: serde_json::Value = serde_json::from_str(&json_str)
                .map_err(|e| mlua::Error::RuntimeError(format!("JSON decode error: {e}")))?;
            json_to_lua_value(lua, &json_val)
        })?,
    )?;

    lua.globals().set("json", json)?;
    Ok(())
}
