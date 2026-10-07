#[cfg(feature = "net")]
use crate::reactive::bridge::ReactiveBridge;
#[cfg(feature = "net")]
use mlua::{Function, Table};
use mlua::{Lua, Result, Value};
#[cfg(feature = "net")]
use parking_lot::Mutex;
#[cfg(feature = "net")]
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
#[cfg(feature = "net")]
use reqwest::{Client, Method};
#[cfg(feature = "net")]
use std::collections::HashMap;
#[cfg(feature = "net")]
use std::str::FromStr;
#[cfg(feature = "net")]
use std::sync::Arc;
#[cfg(feature = "net")]
use std::time::Duration;

#[cfg(feature = "net")]
#[derive(Clone)]
pub struct HttpEngine {
    client: Client,
    bridge: ReactiveBridge,
    lua_ref: Option<Arc<Mutex<Lua>>>,
}

#[cfg(feature = "net")]
impl HttpEngine {
    pub fn new(bridge: ReactiveBridge) -> Self {
        let _guard = crate::tokio_runtime().enter();
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            bridge,
            lua_ref: None,
        }
    }

    pub fn with_lua(mut self, lua: Arc<Mutex<Lua>>) -> Self {
        self.lua_ref = Some(lua);
        self
    }
}

pub fn json_to_lua_value(lua: &Lua, val: &serde_json::Value) -> Result<Value> {
    match val {
        serde_json::Value::Null => Ok(Value::Nil),
        serde_json::Value::Bool(b) => Ok(Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Number(f))
            } else {
                Ok(Value::Nil)
            }
        }
        serde_json::Value::String(s) => Ok(Value::String(lua.create_string(s))),
        serde_json::Value::Array(arr) => {
            let table = lua.create_table();
            for (idx, elem) in arr.iter().enumerate() {
                table.set(idx + 1, json_to_lua_value(lua, elem)?)?;
            }
            Ok(Value::Table(table))
        }
        serde_json::Value::Object(obj) => {
            let table = lua.create_table();
            for (k, v) in obj {
                table.set(k.as_str(), json_to_lua_value(lua, v)?)?;
            }
            Ok(Value::Table(table))
        }
    }
}

#[cfg(feature = "net")]
pub struct HttpResponseData {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[cfg(feature = "net")]
impl HttpResponseData {
    pub fn to_lua_table(&self, lua: &Lua) -> Result<Table> {
        let resp = lua.create_table();
        resp.set("status", self.status)?;
        resp.set("status_text", self.status_text.clone())?;
        resp.set("ok", self.status >= 200 && self.status < 300)?;
        resp.set("body", self.body.clone())?;

        let headers_table = lua.create_table();
        for (k, v) in &self.headers {
            headers_table.set(k.as_str(), v.as_str())?;
        }
        resp.set("headers", headers_table)?;

        let body_clone = self.body.clone();
        let json_fn = lua.create_function(move |lua, ()| {
            match serde_json::from_str::<serde_json::Value>(&body_clone) {
                Ok(val) => json_to_lua_value(lua, &val),
                Err(e) => Err(mlua::Error::RuntimeError(format!("JSON parse error: {e}"))),
            }
        })?;
        resp.set("json", json_fn)?;

        Ok(resp)
    }
}

#[cfg(feature = "net")]
pub fn register(lua: &Lua, engine: HttpEngine) -> Result<()> {
    let http = lua.create_table();

    // Internal request dispatcher
    let dispatch_engine = engine.clone();
    let dispatch_fn = lua.create_function(
        move |lua, (method_str, url_str, opts_val, cb_val): (String, String, Value, Value)| {
            let mut headers_map = HeaderMap::new();
            let mut body_data = None;
            let mut timeout_secs = 30u64;

            let callback: Option<Function> = match (opts_val, cb_val) {
                (Value::Function(f), _) => Some(f),
                (Value::Table(t), Value::Function(f)) => {
                    parse_request_options(&t, &mut headers_map, &mut body_data, &mut timeout_secs);
                    Some(f)
                }
                (Value::Table(t), _) => {
                    parse_request_options(&t, &mut headers_map, &mut body_data, &mut timeout_secs);
                    None
                }
                _ => None,
            };

            let cb_key = if let Some(f) = callback {
                Some(lua.create_registry_value(f)?)
            } else {
                None
            };

            let client = dispatch_engine.client.clone();
            let bridge = dispatch_engine.bridge.clone();
            let lua_holder = dispatch_engine.lua_ref.clone();

            let method = Method::from_str(&method_str.to_uppercase()).unwrap_or(Method::GET);

            let devtools_req_id = crate::devtools::state::record_global_http_request(
                &method_str,
                &url_str,
                headers_map.iter().map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or_default().to_string())).collect(),
                body_data.clone(),
            );

            crate::tokio_runtime().spawn(async move {
                let mut req = client.request(method, &url_str);
                req = req.headers(headers_map);
                req = req.timeout(Duration::from_secs(timeout_secs));

                if let Some(b) = body_data {
                    req = req.body(b);
                }

                let result = match req.send().await {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        let status_text = resp
                            .status()
                            .canonical_reason()
                            .unwrap_or("")
                            .to_string();

                        let mut headers = HashMap::new();
                        for (k, v) in resp.headers() {
                            if let Ok(val_str) = v.to_str() {
                                headers.insert(k.as_str().to_string(), val_str.to_string());
                            }
                        }

                        let body = resp.text().await.unwrap_or_default();
                        Ok(HttpResponseData {
                            status,
                            status_text,
                            headers,
                            body,
                        })
                    }
                    Err(e) => Err(e.to_string()),
                };

                if let Some(req_id) = devtools_req_id {
                    match &result {
                        Ok(data) => {
                            crate::devtools::state::record_global_http_response(
                                req_id,
                                data.status,
                                &data.status_text,
                                data.headers.clone(),
                                Some(data.body.clone()),
                                data.body.len(),
                            );
                        }
                        Err(e) => {
                            crate::devtools::state::record_global_http_response(
                                req_id,
                                500,
                                e,
                                HashMap::new(),
                                None,
                                0,
                            );
                        }
                    }
                }
                if let (Some(key), Some(lua_arc)) = (cb_key, lua_holder) {
                    let lua = lua_arc.lock();
                    if let Ok(func) = lua.registry_value::<Function>(&key) {
                        match result {
                            Ok(resp_data) => {
                                if let Ok(resp_tbl) = resp_data.to_lua_table(&lua) {
                                    let _ = func.call::<()>((Value::Nil, resp_tbl));
                                }
                            }
                            Err(err_msg) => {
                                let _ = func.call::<()>((err_msg, Value::Nil));
                            }
                        }
                    }
                    bridge.notify();
                }
            });

            Ok(())
        },
    )?;

    http.set("__dispatch", dispatch_fn)?;

    // Define http.get, post, put, delete, request, fetch
    lua.load(r#"
        local http_mod = ...

        function http_mod.get(url, opts, cb)
            http_mod.__dispatch("GET", url, opts, cb)
        end

        function http_mod.post(url, opts, cb)
            http_mod.__dispatch("POST", url, opts, cb)
        end

        function http_mod.put(url, opts, cb)
            http_mod.__dispatch("PUT", url, opts, cb)
        end

        function http_mod.delete(url, opts, cb)
            http_mod.__dispatch("DELETE", url, opts, cb)
        end

        function http_mod.request(options, cb)
            local opts = options or {}
            local method = opts.method or "GET"
            local url = opts.url or ""
            http_mod.__dispatch(method, url, opts, cb)
        end

        function http_mod.fetch(url, opts)
            local co = coroutine.running()
            if not co then
                error("http.fetch must be called inside a coroutine")
            end

            http_mod.get(url, opts, function(err, resp)
                local ok, resume_err = coroutine.resume(co, err, resp)
                if not ok then
                    error("Coroutine resume failed: " .. tostring(resume_err))
                end
            end)

            local err, resp = coroutine.yield()
            if err then
                return nil, err
            end
            return resp
        end
    "#).call::<()>(http.clone())?;

    lua.globals().set("http", http)?;
    Ok(())
}

#[cfg(feature = "net")]
fn parse_request_options(
    t: &Table,
    headers: &mut HeaderMap,
    body: &mut Option<String>,
    timeout: &mut u64,
) {
    if let Ok(to) = t.get::<u64>("timeout") {
        *timeout = to;
    }

    if let Ok(headers_tbl) = t.get::<Table>("headers") {
        for pair in headers_tbl.pairs::<String, String>() {
            if let Ok((k, v)) = pair {
                if let (Ok(name), Ok(val)) = (HeaderName::from_str(&k), HeaderValue::from_str(&v)) {
                    headers.insert(name, val);
                }
            }
        }
    }

    if let Ok(b) = t.get::<String>("body") {
        *body = Some(b);
    } else if let Ok(body_tbl) = t.get::<Table>("body") {
        // Encode table to JSON
        if let Ok(json_str) = lua_table_to_json_string(&body_tbl) {
            *body = Some(json_str);
            if !headers.contains_key("content-type") {
                headers.insert(
                    HeaderName::from_static("content-type"),
                    HeaderValue::from_static("application/json"),
                );
            }
        }
    }
}

#[cfg(feature = "net")]
fn lua_table_to_json_string(t: &Table) -> Result<String> {
    let mut map = serde_json::Map::new();
    for pair in t.pairs::<Value, Value>() {
        let (k, v) = pair?;
        let key_str = match k {
            Value::String(s) => s.to_str()?.to_string(),
            Value::Integer(i) => i.to_string(),
            _ => continue,
        };
        let val_json = match v {
            Value::Nil => serde_json::Value::Null,
            Value::Boolean(b) => serde_json::Value::Bool(b),
            Value::Integer(i) => serde_json::Value::Number(i.into()),
            Value::Number(n) => {
                if let Some(num) = serde_json::Number::from_f64(n) {
                    serde_json::Value::Number(num)
                } else {
                    serde_json::Value::Null
                }
            }
            Value::String(s) => serde_json::Value::String(s.to_str()?.to_string()),
            _ => serde_json::Value::Null,
        };
        map.insert(key_str, val_json);
    }
    Ok(serde_json::Value::Object(map).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "net")]
    #[tokio::test]
    async fn test_http_registration_and_methods() -> Result<()> {
        let lua = Lua::new();
        let bridge = ReactiveBridge::new();
        let engine = HttpEngine::new(bridge);
        register(&lua, engine)?;

        lua.load(r#"
            assert(type(http) == "table", "http table not found")
            assert(type(http.get) == "function", "http.get not found")
            assert(type(http.post) == "function", "http.post not found")
            assert(type(http.put) == "function", "http.put not found")
            assert(type(http.delete) == "function", "http.delete not found")
            assert(type(http.request) == "function", "http.request not found")
            assert(type(http.fetch) == "function", "http.fetch not found")
        "#).exec()?;

        Ok(())
    }

    #[test]
    fn test_json_to_lua_conversion() -> Result<()> {
        let lua = Lua::new();
        let json_val: serde_json::Value = serde_json::json!({
            "name": "GPUI.lua",
            "count": 42,
            "active": true,
            "items": ["one", "two"]
        });

        let lua_val = json_to_lua_value(&lua, &json_val)?;
        if let Value::Table(t) = lua_val {
            assert_eq!(t.get::<String>("name")?, "GPUI.lua");
            assert_eq!(t.get::<i64>("count")?, 42);
            assert_eq!(t.get::<bool>("active")?, true);
            let items: Table = t.get("items")?;
            assert_eq!(items.get::<String>(1)?, "one");
            assert_eq!(items.get::<String>(2)?, "two");
        } else {
            panic!("Expected Lua table");
        }
        Ok(())
    }
}
