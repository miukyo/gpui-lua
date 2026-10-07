use base64::Engine;
use mlua::{Lua, Result};
use ring::hmac;
use ring::rand::SecureRandom;
use sha2::{Digest, Sha256, Sha512};
use uuid::Uuid;

pub fn register(lua: &Lua) -> Result<()> {
    let crypto = lua.create_table();

    // crypto.sha256(data) -> hex string
    crypto.set(
        "sha256",
        lua.create_function(|_lua, data: String| {
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            let res = hasher.finalize();
            Ok(format!("{res:x}"))
        })?,
    )?;

    // crypto.sha512(data) -> hex string
    crypto.set(
        "sha512",
        lua.create_function(|_lua, data: String| {
            let mut hasher = Sha512::new();
            hasher.update(data.as_bytes());
            let res = hasher.finalize();
            Ok(format!("{res:x}"))
        })?,
    )?;

    // crypto.hmac_sha256(key, data) -> hex string
    crypto.set(
        "hmac_sha256",
        lua.create_function(|_lua, (key, data): (String, String)| {
            let s_key = hmac::Key::new(hmac::HMAC_SHA256, key.as_bytes());
            let tag = hmac::sign(&s_key, data.as_bytes());
            let hex = tag
                .as_ref()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            Ok(hex)
        })?,
    )?;

    // crypto.uuid() -> string
    crypto.set(
        "uuid",
        lua.create_function(|_lua, ()| Ok(Uuid::new_v4().to_string()))?,
    )?;

    // crypto.random_bytes(len) -> hex string
    crypto.set(
        "random_bytes",
        lua.create_function(|_lua, len: usize| {
            let rng = ring::rand::SystemRandom::new();
            let mut buf = vec![0u8; len];
            let _ = rng.fill(&mut buf);
            let hex = buf.iter().map(|b| format!("{b:02x}")).collect::<String>();
            Ok(hex)
        })?,
    )?;

    // crypto.base64_encode(data) -> string
    crypto.set(
        "base64_encode",
        lua.create_function(|_lua, data: String| {
            Ok(base64::engine::general_purpose::STANDARD.encode(data.as_bytes()))
        })?,
    )?;

    // crypto.base64_decode(data) -> string
    crypto.set(
        "base64_decode",
        lua.create_function(|_lua, data: String| {
            match base64::engine::general_purpose::STANDARD.decode(data.as_bytes()) {
                Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).to_string()),
                Err(e) => Err(mlua::Error::RuntimeError(format!("Base64 decode error: {e}"))),
            }
        })?,
    )?;

    lua.globals().set("crypto", crypto.clone())?;
    if let Ok(pkg) = lua.globals().get::<mlua::Table>("package") {
        if let Ok(loaded) = pkg.get::<mlua::Table>("loaded") {
            let _ = loaded.set("crypto", crypto);
        }
    }

    Ok(())
}
