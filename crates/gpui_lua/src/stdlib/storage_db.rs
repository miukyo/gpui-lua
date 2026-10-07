use mlua::{Lua, Result, Table, Value};
use parking_lot::RwLock;
use rusqlite::{types::ValueRef, Connection};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct LuaDbConnection(pub Arc<parking_lot::Mutex<Option<Connection>>>);

impl mlua::UserData for LuaDbConnection {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        // conn:exec(sql, [params]) -> { changes, last_insert_rowid }
        methods.add_method("exec", |lua, this, (sql, params): (String, Option<Table>)| {
            let guard = this.0.lock();
            let conn = guard
                .as_ref()
                .ok_or_else(|| mlua::Error::RuntimeError("Database connection is closed".to_string()))?;

            let changes = if let Some(p) = params {
                let params_vec = extract_sqlite_params(&p);
                let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec
                    .iter()
                    .map(|v| v as &dyn rusqlite::ToSql)
                    .collect();
                conn.execute(&sql, rusqlite_params.as_slice())
                    .map_err(|e| mlua::Error::RuntimeError(format!("SQLite exec error: {e}")))?
            } else {
                conn.execute(&sql, [])
                    .map_err(|e| mlua::Error::RuntimeError(format!("SQLite exec error: {e}")))?
            };

            let last_id = conn.last_insert_rowid();

            let res_tbl = lua.create_table();
            let _ = res_tbl.set("changes", changes);
            let _ = res_tbl.set("last_insert_rowid", last_id);
            Ok(res_tbl)
        });

        // conn:query(sql, [params]) -> table of row tables
        methods.add_method("query", |lua, this, (sql, params): (String, Option<Table>)| {
            query_internal(this, lua, sql, params)
        });

        // conn:query_row(sql, [params]) -> single row table or nil
        methods.add_method(
            "query_row",
            |lua, this, (sql, params): (String, Option<Table>)| {
                let rows = query_internal(this, lua, sql, params)?;
                if rows.raw_len() > 0 {
                    let row: Table = rows.raw_get(1)?;
                    Ok(Value::Table(row))
                } else {
                    Ok(Value::Nil)
                }
            },
        );

        // conn:close()
        methods.add_method("close", |_lua, this, ()| {
            let mut guard = this.0.lock();
            *guard = None;
            Ok(())
        });
    }
}

fn query_internal(
    this: &LuaDbConnection,
    lua: &Lua,
    sql: String,
    params: Option<Table>,
) -> Result<Table> {
    let guard = this.0.lock();
    let conn = guard
        .as_ref()
        .ok_or_else(|| mlua::Error::RuntimeError("Database connection is closed".to_string()))?;

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| mlua::Error::RuntimeError(format!("SQLite prepare error: {e}")))?;

    let col_names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let rows_data: Vec<Vec<SqlVal>> = if let Some(p) = params {
        let params_vec = extract_sqlite_params(&p);
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec
            .iter()
            .map(|v| v as &dyn rusqlite::ToSql)
            .collect();
        let mut rows = stmt
            .query(rusqlite_params.as_slice())
            .map_err(|e| mlua::Error::RuntimeError(format!("SQLite query error: {e}")))?;
        collect_rows(&mut rows, col_names.len())?
    } else {
        let mut rows = stmt
            .query([])
            .map_err(|e| mlua::Error::RuntimeError(format!("SQLite query error: {e}")))?;
        collect_rows(&mut rows, col_names.len())?
    };

    let out_tbl = lua.create_table();
    for (idx, row) in rows_data.into_iter().enumerate() {
        let row_tbl = lua.create_table();
        for (c_idx, name) in col_names.iter().enumerate() {
            let val = match &row[c_idx] {
                SqlVal::Null => Value::Nil,
                SqlVal::Integer(i) => Value::Integer(*i),
                SqlVal::Real(r) => Value::Number(*r),
                SqlVal::Text(t) => Value::String(lua.create_string(t)),
                SqlVal::Blob(b) => Value::String(lua.create_string(b)),
            };
            let _ = row_tbl.set(name.as_str(), val);
        }
        let _ = out_tbl.set(idx + 1, row_tbl);
    }

    Ok(out_tbl)
}

enum SqlVal {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl rusqlite::ToSql for SqlVal {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            SqlVal::Null => Ok(rusqlite::types::ToSqlOutput::from(rusqlite::types::Null)),
            SqlVal::Integer(i) => Ok(rusqlite::types::ToSqlOutput::from(*i)),
            SqlVal::Real(r) => Ok(rusqlite::types::ToSqlOutput::from(*r)),
            SqlVal::Text(t) => Ok(rusqlite::types::ToSqlOutput::from(t.as_str())),
            SqlVal::Blob(b) => Ok(rusqlite::types::ToSqlOutput::from(b.as_slice())),
        }
    }
}

fn extract_sqlite_params(tbl: &Table) -> Vec<SqlVal> {
    let mut params = Vec::new();
    let len = tbl.raw_len();
    for i in 1..=len {
        if let Ok(val) = tbl.raw_get::<Value>(i) {
            match val {
                Value::Nil => params.push(SqlVal::Null),
                Value::Boolean(b) => params.push(SqlVal::Integer(if b { 1 } else { 0 })),
                Value::Integer(i) => params.push(SqlVal::Integer(i)),
                Value::Number(n) => params.push(SqlVal::Real(n)),
                Value::String(s) => params.push(SqlVal::Text(s.to_str().unwrap_or_default().to_string())),
                _ => params.push(SqlVal::Null),
            }
        }
    }
    params
}

fn collect_rows(rows: &mut rusqlite::Rows, col_count: usize) -> Result<Vec<Vec<SqlVal>>> {
    let mut list = Vec::new();
    while let Ok(Some(row)) = rows.next() {
        let mut cols = Vec::new();
        for i in 0..col_count {
            let val = match row.get_ref(i).unwrap_or(ValueRef::Null) {
                ValueRef::Null => SqlVal::Null,
                ValueRef::Integer(n) => SqlVal::Integer(n),
                ValueRef::Real(r) => SqlVal::Real(r),
                ValueRef::Text(t) => SqlVal::Text(String::from_utf8_lossy(t).to_string()),
                ValueRef::Blob(b) => SqlVal::Blob(b.to_vec()),
            };
            cols.push(val);
        }
        list.push(cols);
    }
    Ok(list)
}

pub fn register(lua: &Lua) -> Result<()> {
    // 1. LocalStorage module (High-speed persistent JSON-backed Key-Value store)
    let storage_data: Arc<RwLock<HashMap<String, serde_json::Value>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let storage_file_path: Arc<RwLock<PathBuf>> = Arc::new(RwLock::new({
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let dir = base.join("gpui-ce");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("localstorage.json");
        // Load existing
        if file.exists() {
            if let Ok(content) = std::fs::read_to_string(&file) {
                if let Ok(map) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&content) {
                    *storage_data.write() = map;
                }
            }
        }
        file
    }));

    let storage = lua.create_table();

    let s_data = storage_data.clone();
    let s_path = storage_file_path.clone();
    storage.set(
        "set",
        lua.create_function(move |_lua, (key, val): (String, Value)| {
            let json_val = crate::stdlib::json::lua_value_to_json(val)?;
            s_data.write().insert(key, json_val);
            // Save to disk
            let path = s_path.read().clone();
            let map = s_data.read().clone();
            let _ = std::fs::write(&path, serde_json::to_string_pretty(&map).unwrap_or_default());
            Ok(())
        })?,
    )?;

    let s_data_g = storage_data.clone();
    storage.set(
        "get",
        lua.create_function(move |lua, (key, default_val): (String, Option<Value>)| {
            let map = s_data_g.read();
            if let Some(json_val) = map.get(&key) {
                crate::net::json_to_lua_value(lua, json_val)
            } else {
                Ok(default_val.unwrap_or(Value::Nil))
            }
        })?,
    )?;

    let s_data_r = storage_data.clone();
    let s_path_r = storage_file_path.clone();
    storage.set(
        "remove",
        lua.create_function(move |_lua, key: String| {
            s_data_r.write().remove(&key);
            let path = s_path_r.read().clone();
            let map = s_data_r.read().clone();
            let _ = std::fs::write(&path, serde_json::to_string_pretty(&map).unwrap_or_default());
            Ok(())
        })?,
    )?;

    let s_data_c = storage_data.clone();
    let s_path_c = storage_file_path.clone();
    storage.set(
        "clear",
        lua.create_function(move |_lua, ()| {
            s_data_c.write().clear();
            let path = s_path_c.read().clone();
            let _ = std::fs::write(&path, "{}");
            Ok(())
        })?,
    )?;

    let s_data_k = storage_data.clone();
    storage.set(
        "keys",
        lua.create_function(move |lua, ()| {
            let map = s_data_k.read();
            let tbl = lua.create_table();
            for (idx, k) in map.keys().enumerate() {
                let _ = tbl.set(idx + 1, k.as_str());
            }
            Ok(tbl)
        })?,
    )?;

    let s_data_all = storage_data.clone();
    storage.set(
        "all",
        lua.create_function(move |lua, ()| {
            let map = s_data_all.read();
            let tbl = lua.create_table();
            for (k, v) in map.iter() {
                let _ = tbl.set(k.as_str(), crate::net::json_to_lua_value(lua, v)?);
            }
            Ok(tbl)
        })?,
    )?;

    lua.globals().set("storage", storage.clone())?;

    // 2. Database (SQLite) module
    let db = lua.create_table();

    // db.open(path) -> connection
    db.set(
        "open",
        lua.create_function(|_lua, path: String| {
            let conn = Connection::open(&path)
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to open SQLite database: {e}")))?;
            Ok(LuaDbConnection(Arc::new(parking_lot::Mutex::new(Some(conn)))))
        })?,
    )?;

    // db.open_memory() -> in-memory connection
    db.set(
        "open_memory",
        lua.create_function(|_lua, ()| {
            let conn = Connection::open_in_memory()
                .map_err(|e| mlua::Error::RuntimeError(format!("Failed to open in-memory SQLite: {e}")))?;
            Ok(LuaDbConnection(Arc::new(parking_lot::Mutex::new(Some(conn)))))
        })?,
    )?;

    lua.globals().set("db", db.clone())?;

    if let Ok(pkg) = lua.globals().get::<Table>("package") {
        if let Ok(loaded) = pkg.get::<Table>("loaded") {
            let _ = loaded.set("storage", storage);
            let _ = loaded.set("db", db);
        }
    }

    Ok(())
}
