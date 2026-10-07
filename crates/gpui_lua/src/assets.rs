use mlua::{Function, Lua, Result, Table, Value};
use parking_lot::RwLock;
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::sync::Arc;

#[derive(RustEmbed)]
#[folder = "src/stdlib/lua/"]
pub struct EmbeddedStdlib;

#[derive(RustEmbed)]
#[folder = "types/"]
pub struct EmbeddedTypes;

pub type AssetLookupFn = Arc<dyn Fn(&str) -> Option<Cow<'static, [u8]>> + Send + Sync>;

#[derive(Clone, Default)]
pub struct EmbeddedAssetManager {
    providers: Arc<RwLock<Vec<AssetLookupFn>>>,
    filenames: Arc<RwLock<Vec<gpui::SharedString>>>,
}

impl EmbeddedAssetManager {
    pub fn new() -> Self {
        let manager = Self {
            providers: Arc::new(RwLock::new(Vec::new())),
            filenames: Arc::new(RwLock::new(Vec::new())),
        };

        // Always register built-in stdlib and types
        manager.register_embed::<EmbeddedStdlib>();
        manager.register_embed::<EmbeddedTypes>();

        manager
    }

    /// Register a `RustEmbed` asset struct with the asset manager.
    pub fn register_embed<E: RustEmbed + 'static>(&self) {
        let lookup: AssetLookupFn = Arc::new(|path: &str| {
            let normalized = path.replace('\\', "/");
            let clean = normalized.trim_start_matches('/');
            E::get(clean).map(|f| f.data)
        });
        self.providers.write().push(lookup);

        let mut names = self.filenames.write();
        for file in E::iter() {
            let s = file.as_ref().replace('\\', "/");
            names.push(gpui::SharedString::from(s));
        }
    }
    /// Register a map of in-memory files (e.g. from bundled standalone package).
    pub fn register_file_map(&self, files: std::collections::HashMap<String, Vec<u8>>) {
        let files = Arc::new(files);
        let files_clone = Arc::clone(&files);
        let lookup: AssetLookupFn = Arc::new(move |path: &str| {
            let normalized = path.replace('\\', "/");
            let clean = normalized.trim_start_matches('/');
            files_clone.get(clean).cloned().map(Cow::Owned)
        });
        self.providers.write().push(lookup);

        let mut names = self.filenames.write();
        for k in files.keys() {
            names.push(gpui::SharedString::from(k.clone()));
        }
    }
    /// Get raw asset bytes by path from any registered embedded provider.
    pub fn get(&self, path: &str) -> Option<Cow<'static, [u8]>> {
        let providers = self.providers.read();
        for provider in providers.iter().rev() {
            if let Some(data) = provider(path) {
                return Some(data);
            }
        }
        None
    }

    /// Get UTF-8 string content of an embedded asset.
    pub fn get_str(&self, path: &str) -> Option<String> {
        self.get(path).and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
    }

    /// Hook an embedded asset module loader into Lua's `package.loaders` / `package.searchers`.
    pub fn register_lua_searcher(&self, lua: &Lua) -> Result<()> {
        let manager = self.clone();

        let searcher_fn = lua.create_function(move |lua, module_name: String| {
            let rel_path = module_name.replace('.', "/");
            let candidates = [
                format!("{rel_path}.lua"),
                format!("{rel_path}/init.lua"),
                rel_path.clone(),
            ];

            for candidate in candidates {
                if let Some(source) = manager.get_str(&candidate) {
                    let chunk_name = format!("@embedded:{}", candidate);
                    let loader = lua.load(&source).set_name(&chunk_name).into_function()?;
                    return Ok((Value::Function(loader), Value::Nil));
                }
            }

            Ok((
                Value::Nil,
                Value::String(lua.create_string(&format!(
                    "\n\tno embedded asset found for module '{}'",
                    module_name
                ))),
            ))
        })?;

        // In Lua 5.1 / LuaJIT it's package.loaders, in Lua 5.2+ it's package.searchers
        let package: Table = match lua.globals().get("package") {
            Ok(p) => p,
            Err(_) => {
                let p = lua.create_table();
                let loaders = lua.create_table();
                p.set("loaders", loaders.clone())?;
                p.set("searchers", loaders)?;
                lua.globals().set("package", p.clone())?;
                p
            }
        };
        let searchers: Option<Table> = package.get("loaders").ok().or_else(|| package.get("searchers").ok());

        if let Some(searchers_table) = searchers {
            let len = searchers_table.raw_len();
            let mut list: Vec<Function> = Vec::new();
            for i in 1..=len {
                if let Ok(f) = searchers_table.raw_get::<Function>(i) {
                    list.push(f);
                }
            }
            searchers_table.raw_set(1, list.first().cloned().unwrap_or(searcher_fn.clone()))?;
            searchers_table.raw_set(2, searcher_fn)?;
            for (idx, f) in list.into_iter().skip(1).enumerate() {
                searchers_table.raw_set(idx + 3, f)?;
            }
        }

        // Expose `embedded` global table for Lua
        let embedded_tbl = lua.create_table();
        let mgr = self.clone();
        embedded_tbl.set(
            "read",
            lua.create_function(move |lua, path: String| {
                if let Some(content) = mgr.get_str(&path) {
                    Ok(Value::String(lua.create_string(&content)))
                } else {
                    Ok(Value::Nil)
                }
            })?,
        )?;

        let mgr_exists = self.clone();
        embedded_tbl.set(
            "exists",
            lua.create_function(move |_lua, path: String| {
                Ok(mgr_exists.get(&path).is_some())
            })?,
        )?;

        lua.globals().set("embedded", embedded_tbl)?;

        Ok(())
    }
}

impl gpui::AssetSource for EmbeddedAssetManager {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(self.get(path))
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        Ok(self.filenames.read().clone())
    }
}
