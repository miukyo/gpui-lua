pub mod error_view;
pub mod watcher;

pub use error_view::{HotReloadError, render_error_view};
pub use watcher::{ReloadCallback, ScriptWatcher};

use mlua::Lua;
use std::path::Path;

pub fn reload_lua_file(lua: &Lua, path: &Path) -> std::result::Result<(), HotReloadError> {
    let script_content = std::fs::read_to_string(path).map_err(|e| {
        HotReloadError::new(format!("Failed to read {}: {e}", path.display()))
    })?;

    // Invalidate package.loaded entries (preserving core tables)
    let invalidate_fn = r#"
        local protected = {
            _G = true, package = true, string = true, table = true,
            math = true, io = true, os = true, debug = true, bit = true,
            jit = true, coroutine = true, ui = true, reactive = true,
            http = true, timer = true, json = true, fs = true, log = true,
        }
        if package and package.loaded then
            for k in pairs(package.loaded) do
                if not protected[k] then
                    package.loaded[k] = nil
                end
            end
        end
    "#;
    let _ = lua.load(invalidate_fn).exec();

    // Execute with xpcall to capture traceback
    let chunk_name = path.to_string_lossy().to_string();
    let res: mlua::Result<(bool, Option<String>, Option<String>)> = lua
        .load(&format!(
            r#"
            local ok, err = xpcall(function()
                local res = (function()
                    {}
                end)()
                if res ~= nil then
                    __gpui_last_result = res
                end
            end, function(e)
                return tostring(e) .. "\n" .. debug.traceback("", 2)
            end)

            if not ok then
                return false, tostring(err), debug.traceback()
            end
            return true, nil, nil
            "#,
            script_content
        ))
        .set_name(&chunk_name)
        .eval();

    match res {
        Ok((true, _, _)) => Ok(()),
        Ok((false, Some(err_msg), trace)) => {
            Err(HotReloadError::parse(&err_msg, trace.as_deref()))
        }
        Ok((false, None, trace)) => {
            Err(HotReloadError::parse("Unknown Lua error during reload", trace.as_deref()))
        }
        Err(e) => {
            let err_str = e.to_string();
            Err(HotReloadError::parse(&err_str, None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_view_construction() {
        let err = HotReloadError::parse("src/app.lua:42: attempt to call nil", Some("stack traceback:\n\tin function render"));
        assert_eq!(err.file.as_deref(), Some("src/app.lua"));
        assert_eq!(err.line, Some(42));
        assert!(err.traceback.is_some());

        let _el = render_error_view(&err);
    }

    #[test]
    fn test_reload_syntax_error() {
        let lua = Lua::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("bad.lua");
        std::fs::write(&file_path, "function broken() invalid syntax end").unwrap();

        let result = reload_lua_file(&lua, &file_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("syntax") || err.message.contains("<eof>"));
    }

    #[test]
    fn test_reload_success() {
        let lua = Lua::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("good.lua");
        std::fs::write(&file_path, "global_val = 123").unwrap();

        let result = reload_lua_file(&lua, &file_path);
        assert!(result.is_ok());

        let val: i32 = lua.globals().get("global_val").unwrap();
        assert_eq!(val, 123);
    }
}
