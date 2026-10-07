use gpui::AnyElement;
use parking_lot::RwLock;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

/// Context passed to custom element rendering closures.
pub struct CustomElementContext {
    pub tag: String,
    pub props: JsonValue,
    pub children: Vec<AnyElement>,
}

impl CustomElementContext {
    /// Helper to get a string property by key.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.props.get(key).and_then(|v| v.as_str())
    }

    /// Helper to get a boolean property by key.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.props.get(key).and_then(|v| v.as_bool())
    }

    /// Helper to get an f64 property by key.
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.props.get(key).and_then(|v| v.as_f64())
    }

    /// Helper to get an i64 property by key.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.props.get(key).and_then(|v| v.as_i64())
    }

    /// Helper to deserialize a specific property to any Serde type.
    pub fn get_prop<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.props.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Helper to deserialize the entire props object into a custom struct.
    pub fn deserialize<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.props.clone())
    }
}

pub type CustomElementRenderer = Arc<dyn Fn(CustomElementContext) -> AnyElement + Send + Sync>;

static REGISTRY: LazyLock<RwLock<HashMap<String, CustomElementRenderer>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Registers a custom GPUI element renderer by tag name.
pub fn register_custom_element(tag: &str, renderer: CustomElementRenderer) {
    REGISTRY.write().insert(tag.to_string(), renderer);
}

/// Retrieves a registered custom element renderer by tag name.
pub fn get_custom_element(tag: &str) -> Option<CustomElementRenderer> {
    REGISTRY.read().get(tag).cloned()
}

/// Returns the names of all registered custom elements.
pub fn custom_element_names() -> Vec<String> {
    REGISTRY.read().keys().cloned().collect()
}

/// Binds a registered custom element into a Lua state under `ui.<tag>(props_or_children)`.
pub fn bind_custom_element_in_lua(lua: &mlua::Lua, tag: &str) -> mlua::Result<()> {
    let ui: mlua::Table = match lua.globals().get("ui") {
        Ok(t) => t,
        Err(_) => return Ok(()),
    };

    let tag_name = tag.to_string();
    ui.set(
        tag,
        lua.create_function(move |lua, arg: Option<mlua::Value>| {
            let mut props_map = serde_json::Map::new();
            let mut children = Vec::new();

            if let Some(val) = arg {
                match val {
                    mlua::Value::Table(tbl) => {
                        // Check if it's an array of children or a props table
                        for pair in tbl.pairs::<mlua::Value, mlua::Value>() {
                            if let Ok((k, v)) = pair {
                                match k {
                                    mlua::Value::Integer(_) => {
                                        if let Some(node) = crate::dsl::builder::value_to_node(lua, v) {
                                            children.push(node);
                                        }
                                    }
                                    mlua::Value::String(k_str) => {
                                        if let Ok(key) = k_str.to_str() {
                                            if key == "children" {
                                                if let mlua::Value::Table(ch_tbl) = v {
                                                    for ch in ch_tbl.sequence_values::<mlua::Value>() {
                                                        if let Ok(ch_val) = ch {
                                                            if let Some(node) = crate::dsl::builder::value_to_node(lua, ch_val) {
                                                                children.push(node);
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                props_map.insert(key.to_string(), lua_value_to_json(&v));
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    other => {
                        props_map.insert("value".to_string(), lua_value_to_json(&other));
                    }
                }
            }

            let builder = crate::dsl::builder::LuaElementBuilder::new_custom(
                tag_name.clone(),
                JsonValue::Object(props_map),
            );

            if !children.is_empty() {
                builder.with_custom_mut(|c| {
                    c.children = children;
                });
            }

            Ok(builder)
        })?,
    )?;

    Ok(())
}

/// Converts an mlua::Value recursively into a serde_json::Value.
pub fn lua_value_to_json(val: &mlua::Value) -> JsonValue {
    match val {
        mlua::Value::Nil => JsonValue::Null,
        mlua::Value::Boolean(b) => JsonValue::Bool(*b),
        mlua::Value::Integer(i) => JsonValue::Number((*i).into()),
        mlua::Value::Number(n) => serde_json::Number::from_f64(*n)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        mlua::Value::String(s) => {
            JsonValue::String(s.to_str().map(|s| s.to_string()).unwrap_or_default())
        }
        mlua::Value::Table(t) => {
            let is_array = t.raw_len() > 0;
            if is_array {
                let mut arr = Vec::new();
                for v in t.sequence_values::<mlua::Value>() {
                    if let Ok(v) = v {
                        arr.push(lua_value_to_json(&v));
                    }
                }
                JsonValue::Array(arr)
            } else {
                let mut map = serde_json::Map::new();
                for pair in t.pairs::<mlua::Value, mlua::Value>() {
                    if let Ok((k, v)) = pair {
                        if let mlua::Value::String(k_str) = k {
                            if let Ok(k_s) = k_str.to_str() {
                                map.insert(k_s.to_string(), lua_value_to_json(&v));
                            }
                        }
                    }
                }
                JsonValue::Object(map)
            }
        }
        _ => JsonValue::Null,
    }
}
