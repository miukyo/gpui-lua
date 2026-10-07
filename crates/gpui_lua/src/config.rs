use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Application configuration file representation (e.g. `gpui.toml` or `gpui.json`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default, alias = "package")]
    pub app: PackageConfig,
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default)]
    pub dev: DevConfig,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: Option<String>,
    pub entry: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub product_name: Option<String>,
    pub file_description: Option<String>,
    pub description: Option<String>,
    pub company_name: Option<String>,
    pub copyright: Option<String>,
    pub icon: Option<String>,
    pub identifier: Option<String>,
}

impl PackageConfig {
    pub fn resolved_product_name(&self) -> Option<String> {
        self.product_name.clone().or_else(|| self.name.clone())
    }

    pub fn resolved_file_description(&self) -> Option<String> {
        self.file_description.clone().or_else(|| self.description.clone())
    }

    pub fn resolved_company_name(&self) -> Option<String> {
        self.company_name.clone().or_else(|| self.author.clone())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowConfig {
    pub title: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
    pub resizable: Option<bool>,
    pub minimizable: Option<bool>,
    pub csd: Option<bool>,
    pub csd_height: Option<f32>,

    /// Universal background or per-OS background table (`[window.background]`).
    #[serde(default)]
    pub background: Option<WindowBackgroundConfig>,

    /// Specific Windows background override ("transparent", "mica", "mica-alt", "acrylic", "opaque").
    #[serde(default)]
    pub windows_background: Option<String>,

    /// Specific macOS background / NSVisualEffectMaterial override ("sidebar", "titlebar", "hud", etc.).
    #[serde(default)]
    pub macos_background: Option<String>,

    /// Specific Linux background override ("transparent", "opaque").
    #[serde(default)]
    pub linux_background: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WindowBackgroundConfig {
    Single(String),
    Platform(PlatformBackgroundConfig),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlatformBackgroundConfig {
    pub windows: Option<String>,
    pub macos: Option<String>,
    pub linux: Option<String>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: None,
            width: Some(900.0),
            height: Some(650.0),
            min_width: Some(400.0),
            min_height: Some(300.0),
            resizable: Some(true),
            minimizable: Some(true),
            csd: Some(true),
            csd_height: Some(38.0),
            background: None,
            windows_background: None,
            macos_background: None,
            linux_background: None,
        }
    }
}

impl WindowConfig {
    /// Resolves the active background string for the current operating system.
    pub fn active_background(&self) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            if let Some(ref bg) = self.windows_background {
                return Some(bg.clone());
            }
            if let Some(WindowBackgroundConfig::Platform(ref p)) = self.background {
                if let Some(ref bg) = p.windows {
                    return Some(bg.clone());
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(ref bg) = self.macos_background {
                return Some(bg.clone());
            }
            if let Some(WindowBackgroundConfig::Platform(ref p)) = self.background {
                if let Some(ref bg) = p.macos {
                    return Some(bg.clone());
                }
            }
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            if let Some(ref bg) = self.linux_background {
                return Some(bg.clone());
            }
            if let Some(WindowBackgroundConfig::Platform(ref p)) = self.background {
                if let Some(ref bg) = p.linux {
                    return Some(bg.clone());
                }
            }
        }

        match &self.background {
            Some(WindowBackgroundConfig::Single(s)) => Some(s.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BuildConfig {
    pub output: Option<PathBuf>,
    pub assets: Option<PathBuf>,
    pub template: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevConfig {
    pub hot_reload: Option<bool>,
}

impl Default for DevConfig {
    fn default() -> Self {
        Self {
            hot_reload: Some(true),
        }
    }
}

impl AppConfig {
    /// Loads configuration from a specific file path.
    pub fn load_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let config: Self = serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse JSON config in {}", path.display()))?;
            Ok(config)
        } else {
            let config: Self = toml::from_str(&content)
                .with_context(|| format!("Failed to parse TOML config in {}", path.display()))?;
            Ok(config)
        }
    }

    /// Automatically discovers and loads a configuration file in `dir` (or cwd if none).
    /// Looks for `gpui.toml`, `gpui.lua.toml`, or `gpui.json`.
    pub fn find_in(dir: Option<&Path>) -> Option<(Self, PathBuf)> {
        let base_dir = dir
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let candidates = [
            base_dir.join("gpui.toml"),
            base_dir.join("gpui.lua.toml"),
            base_dir.join("gpui.json"),
        ];

        for candidate in candidates {
            if candidate.is_file() {
                if let Ok(config) = Self::load_file(&candidate) {
                    return Some((config, candidate));
                }
            }
        }

        None
    }
    /// Applies window configuration to a `LuaApp`.
    pub fn apply_to_app(&self, mut app: crate::app::LuaApp) -> crate::app::LuaApp {
        if let Some(t) = &self.window.title {
            app = app.title(t);
        } else if let Some(n) = &self.app.name {
            app = app.title(n);
        }

        if let (Some(w), Some(h)) = (self.window.width, self.window.height) {
            app = app.size(w, h);
        }
        if let (Some(mw), Some(mh)) = (self.window.min_width, self.window.min_height) {
            app = app.min_size(mw, mh);
        }
        if let Some(r) = self.window.resizable {
            app = app.resizable(r);
        }
        if let Some(m) = self.window.minimizable {
            app = app.minimizable(m);
        }
        if self.window.csd.unwrap_or(true) {
            app = app.csd();
        }
        if let Some(h) = self.window.csd_height {
            let mut opts = app.csd_options;
            opts.height = h;
            app = app.csd_options(opts);
        }
        #[cfg(target_os = "windows")]
        if let Some(bg) = self.window.active_background() {
            match bg.to_lowercase().trim() {
                "acrylic" => { app = app.windows_background(gpui::WindowsWindowBackground::Acrylic); }
                "mica" => { app = app.windows_background(gpui::WindowsWindowBackground::MicaBackdrop); }
                "mica_alt" | "mica-alt" | "tabbed" => { app = app.windows_background(gpui::WindowsWindowBackground::MicaAltBackdrop); }
                "opaque" => { app = app.windows_background(gpui::WindowsWindowBackground::Opaque); }
                "transparent" => { app = app.windows_background(gpui::WindowsWindowBackground::Transparent); }
                "blurred" => { app = app.windows_background(gpui::WindowsWindowBackground::Blurred); }
                other => {
                    log::warn!("Unknown Windows background '{other}', expected: transparent, mica, mica-alt, acrylic, opaque");
                }
            }
        }

        #[cfg(target_os = "macos")]
        if let Some(bg) = self.window.active_background() {
            app = app.set_macos_background_str(&bg);
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        if let Some(bg) = self.window.active_background() {
            match bg.to_lowercase().trim() {
                "transparent" => {
                    app = app.linux_background(gpui::LinuxWindowBackground::Transparent);
                }
                "opaque" => {
                    app = app.linux_background(gpui::LinuxWindowBackground::Opaque);
                }
                "blurred" => {
                    app = app.linux_background(gpui::LinuxWindowBackground::Blurred);
                }
                other => {
                    log::warn!("Unknown Linux background '{other}', expected: transparent or opaque");
                }
            }
        }

        app
    }

    /// Generates a starter `gpui.toml` string.
    pub fn starter_template(name: &str, entry: &str) -> String {
        format!(
            r#"[app]
name = "{name}"
version = "0.1.0"
entry = "{entry}"
product_name = "{name}"
file_description = "{name} Desktop Application"
company_name = "Acme Corp"
copyright = "Copyright (c) 2026"
identifier = "com.example.{name}"
icon = "assets/icon.ico"

[window]
title = "{name}"
height = 650
min_width = 400
min_height = 300
resizable = true
minimizable = true
csd = true
csd_height = 38.0

# OS-dependent window background
[window.background]
windows = "acrylic"       # transparent, mica, mica-alt, acrylic, opaque
macos = "sidebar"         # any NSVisualEffectMaterial (sidebar, titlebar, hud, etc.) or transparent, opaque
linux = "transparent"     # transparent, opaque

[build]
output = "dist/{name}.exe"
assets = "assets"

[dev]
hot_reload = true
"#
        )
    }
}
