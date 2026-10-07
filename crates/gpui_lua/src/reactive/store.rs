use mlua::{Lua, Result, Value};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub enum StoredValue {
    Nil,
    Boolean(bool),
    Integer(i64),
    Number(f64),
    String(String),
    Json(serde_json::Value),
}

impl StoredValue {
    pub fn from_lua_value(val: &Value) -> Self {
        match val {
            Value::Nil => StoredValue::Nil,
            Value::Boolean(b) => StoredValue::Boolean(*b),
            Value::Integer(i) => StoredValue::Integer(*i),
            Value::Number(n) => StoredValue::Number(*n),
            Value::String(s) => {
                let str_val = s.to_str().map(|v| v.to_string()).unwrap_or_default();
                StoredValue::String(str_val)
            }
            Value::Table(t) => {
                if let Ok(json_val) = crate::stdlib::json::lua_value_to_json(Value::Table(t.clone())) {
                    return StoredValue::Json(json_val);
                }
                StoredValue::Nil
            }
            _ => StoredValue::Nil,
        }
    }

    pub fn to_lua_value(&self, lua: &Lua) -> Result<Value> {
        match self {
            StoredValue::Nil => Ok(Value::Nil),
            StoredValue::Boolean(b) => Ok(Value::Boolean(*b)),
            StoredValue::Integer(i) => Ok(Value::Integer(*i)),
            StoredValue::Number(n) => Ok(Value::Number(*n)),
            StoredValue::String(s) => Ok(Value::String(lua.create_string(s))),
            StoredValue::Json(j) => crate::net::json_to_lua_value(lua, j),
        }
    }
}

#[derive(Clone, Default)]
pub struct ReactiveStore {
    values: Arc<RwLock<HashMap<String, StoredValue>>>,
}

impl ReactiveStore {
    pub fn new() -> Self {
        Self {
            values: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get(&self, key: &str) -> Option<StoredValue> {
        self.values.read().get(key).cloned()
    }

    pub fn set(&self, key: String, val: StoredValue) {
        self.values.write().insert(key, val);
    }

    pub fn has(&self, key: &str) -> bool {
        self.values.read().contains_key(key)
    }

    pub fn clear(&self) {
        self.values.write().clear();
    }

    pub fn keys(&self) -> Vec<String> {
        self.values.read().keys().cloned().collect()
    }
}
