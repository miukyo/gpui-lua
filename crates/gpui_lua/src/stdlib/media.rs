#![allow(unused_variables, unused_imports)]
use mlua::{Lua, Result, Table, UserData, UserDataMethods, Value};
use std::sync::Arc;

#[cfg(feature = "media")]
use media::{CameraCapture, CameraOptions, MicrophoneCapture, MicrophoneOptions, VideoManager, AudioManager};

#[derive(Clone)]
pub struct LuaCameraCapture {
    #[cfg(feature = "media")]
    pub inner: Arc<CameraCapture>,
}

impl UserData for LuaCameraCapture {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("src", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.src().to_string());
            #[cfg(not(feature = "media"))]
            Ok("camera://0".to_string())
        });

        methods.add_method("uri", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.uri().to_string());
            #[cfg(not(feature = "media"))]
            Ok("camera://0".to_string())
        });

        methods.add_method("name", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.name().to_string());
            #[cfg(not(feature = "media"))]
            Ok("Default Live Camera".to_string())
        });

        methods.add_method("id", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.id());
            #[cfg(not(feature = "media"))]
            Ok(0)
        });

        methods.add_method("width", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.width());
            #[cfg(not(feature = "media"))]
            Ok(1280)
        });

        methods.add_method("height", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.height());
            #[cfg(not(feature = "media"))]
            Ok(720)
        });

        methods.add_method("fps", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.fps());
            #[cfg(not(feature = "media"))]
            Ok(30)
        });

        methods.add_method("mute", |_lua, this, muted: bool| {
            #[cfg(feature = "media")]
            this.inner.set_muted(muted);
            Ok(())
        });

        methods.add_method("is_muted", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.is_muted());
            #[cfg(not(feature = "media"))]
            Ok(false)
        });

        methods.add_method("stop", |_lua, this, ()| {
            #[cfg(feature = "media")]
            this.inner.stop();
            Ok(())
        });
    }
}

impl mlua::FromLua for LuaCameraCapture {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "LuaCameraCapture".to_string(),
                message: None,
            }),
        }
    }
}

#[derive(Clone)]
pub struct LuaMicrophoneCapture {
    #[cfg(feature = "media")]
    pub inner: Arc<MicrophoneCapture>,
}

impl UserData for LuaMicrophoneCapture {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("src", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.src().to_string());
            #[cfg(not(feature = "media"))]
            Ok("microphone://0".to_string())
        });

        methods.add_method("uri", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.uri().to_string());
            #[cfg(not(feature = "media"))]
            Ok("microphone://0".to_string())
        });

        methods.add_method("name", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.name().to_string());
            #[cfg(not(feature = "media"))]
            Ok("Default Audio Input".to_string())
        });

        methods.add_method("id", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.id());
            #[cfg(not(feature = "media"))]
            Ok(0)
        });

        methods.add_method("mute", |_lua, this, muted: bool| {
            #[cfg(feature = "media")]
            this.inner.set_muted(muted);
            Ok(())
        });

        methods.add_method("is_muted", |_lua, this, ()| {
            #[cfg(feature = "media")]
            return Ok(this.inner.is_muted());
            #[cfg(not(feature = "media"))]
            Ok(false)
        });

        methods.add_method("stop", |_lua, this, ()| {
            #[cfg(feature = "media")]
            this.inner.stop();
            Ok(())
        });
    }
}

impl mlua::FromLua for LuaMicrophoneCapture {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "LuaMicrophoneCapture".to_string(),
                message: None,
            }),
        }
    }
}

#[allow(unused_variables)]
pub fn register(
    lua: &Lua,
    #[cfg(feature = "media")] video: VideoManager,
    #[cfg(feature = "media")] audio: AudioManager,
) -> Result<()> {
    let media_tbl = lua.create_table();

    #[cfg(feature = "media")]
    {
        let v_mgr = video.clone();
        media_tbl.set(
            "open_camera",
            lua.create_function(move |_lua, opts: Option<Table>| {
                let mut options = CameraOptions::default();
                if let Some(t) = opts {
                    if let Ok(dev) = t.get::<usize>("device") {
                        options.device_index = dev;
                    }
                    if let Ok(name) = t.get::<String>("name").or_else(|_| t.get::<String>("device_name")) {
                        options.device_name = Some(name);
                    }
                    if let Ok(w) = t.get::<u32>("width") {
                        options.width = w;
                    }
                    if let Ok(h) = t.get::<u32>("height") {
                        options.height = h;
                    }
                    if let Ok(fps) = t.get::<u32>("fps") {
                        options.fps = fps;
                    }
                }

                let cam = CameraCapture::start(options)
                    .map_err(|e| mlua::Error::RuntimeError(format!("Failed to open camera: {e}")))?;

                // Automatically bind to local video manager stream player for instant preview in ui.video
                let player = v_mgr.register_stream_player(cam.src());
                cam.bind_player(player);

                Ok(LuaCameraCapture { inner: cam })
            })?,
        )?;


        let a_mgr = audio.clone();
        media_tbl.set(
            "open_microphone",
            lua.create_function(move |_lua, opts: Option<Table>| {
                let mut options = MicrophoneOptions::default();
                if let Some(t) = opts {
                    if let Ok(dev) = t.get::<usize>("device") {
                        options.device_index = dev;
                    }
                    if let Ok(name) = t.get::<String>("name").or_else(|_| t.get::<String>("device_name")) {
                        options.device_name = Some(name);
                    }
                    if let Ok(sr) = t.get::<u32>("sample_rate") {
                        options.sample_rate = sr;
                    }
                    if let Ok(ch) = t.get::<u16>("channels") {
                        options.channels = ch;
                    }
                }

                let mic = MicrophoneCapture::start(options)
                    .map_err(|e| mlua::Error::RuntimeError(format!("Failed to open microphone: {e}")))?;

                let player = a_mgr.register_stream_player(mic.src());
                mic.bind_player(player);

                Ok(LuaMicrophoneCapture { inner: mic })
            })?,
        )?;
    }

    media_tbl.set(
        "list_cameras",
        lua.create_function(|lua, ()| {
            let list = lua.create_table();
            #[cfg(feature = "media")]
            {
                let devices = media::list_camera_devices();
                for (idx, dev) in devices.into_iter().enumerate() {
                    let cam_tbl = lua.create_table();
                    cam_tbl.set("id", dev.id)?;
                    cam_tbl.set("name", dev.name)?;
                    cam_tbl.set("default", dev.is_default)?;
                    list.set(idx + 1, cam_tbl)?;
                }
            }
            Ok(list)
        })?,
    )?;

    media_tbl.set(
        "list_microphones",
        lua.create_function(|lua, ()| {
            let list = lua.create_table();
            #[cfg(feature = "media")]
            {
                let devices = media::list_microphone_devices();
                for (idx, dev) in devices.into_iter().enumerate() {
                    let mic_tbl = lua.create_table();
                    mic_tbl.set("id", dev.id)?;
                    mic_tbl.set("name", dev.name)?;
                    mic_tbl.set("default", dev.is_default)?;
                    list.set(idx + 1, mic_tbl)?;
                }
            }
            Ok(list)
        })?,
    )?;

    // media.bind(source, target) helper: binds camera/track to video element or stream player
    media_tbl.set(
        "bind",
        lua.create_function(|_lua, (source, _target): (Value, Value)| {
            let src_str = match &source {
                Value::String(s) => s.to_str()?.to_string(),
                Value::UserData(ud) => {
                    #[cfg(feature = "media")]
                    if let Ok(cam) = ud.borrow::<LuaCameraCapture>() {
                        cam.inner.src().to_string()
                    } else if let Ok(mic) = ud.borrow::<LuaMicrophoneCapture>() {
                        mic.inner.src().to_string()
                    } else {
                        return Err(mlua::Error::RuntimeError("Unknown media source for binding".to_string()));
                    }
                    #[cfg(not(feature = "media"))]
                    return Err(mlua::Error::RuntimeError("Media feature not enabled".to_string()));
                }
                _ => return Err(mlua::Error::RuntimeError("Invalid media source".to_string())),
            };
            Ok(src_str)
        })?,
    )?;

    lua.globals().set("media", media_tbl.clone())?;

    if let Ok(pkg) = lua.globals().get::<Table>("package") {
        if let Ok(loaded) = pkg.get::<Table>("loaded") {
            let _ = loaded.set("media", media_tbl);
        }
    }

    Ok(())
}
