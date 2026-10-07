use gpui_lua::bundle::{
    bundle_package, check_bundled_app, collect_dir_files_public, AppBundleManifest,
};
use gpui_lua::{AppConfig, LuaApp};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = r#"GPUI.lua - High-performance GPU-accelerated Lua desktop runtime and bundler.

USAGE:
    gpui-lua <COMMAND> [OPTIONS]
    gpui-lua <SCRIPT.lua> [OPTIONS]

COMMANDS:
    init [NAME]               Create a starter gpui.toml and main.lua
    run [SCRIPT.lua]          Run Lua application (auto-detects gpui.toml)
    dev [SCRIPT.lua]          Run with live hot-reloading file watcher enabled
    build [ENTRY.lua]         Bundle application into a standalone single executable
    version, -v, --version    Show version and engine information
    help, -h, --help          Show this help message

OPTIONS:
    -c, --config <PATH>       Path to config file (default: gpui.toml)
    -o, --output <PATH>       Output executable path for build
    -a, --assets <DIR>        Assets directory to bundle alongside script
    -t, --title <STRING>      Initial window title
    -w, --width <NUMBER>      Initial window width in pixels
    -h, --height <NUMBER>     Initial window height in pixels
    --min-width <NUMBER>      Minimum window width constraint
    --min-height <NUMBER>     Minimum window height constraint
    --csd / --no-csd          Enable or disable custom Client-Side Decorations
    --csd-height <NUMBER>     Height of CSD titlebar in pixels (default: 38)
    --background <VALUE>      Window background material (acrylic, mica, sidebar, etc.)
    --windows-background <V>  Windows background (transparent, mica, mica-alt, acrylic, opaque)
    --macos-background <V>    macOS background (NSVisualEffectMaterial, transparent, opaque)
    --linux-background <V>    Linux background (transparent, opaque)
    --template <PATH>         Base template executable to package onto
    --icon <PATH>             Application icon (.ico, .icns, .png)
    --product-name <STRING>   Product name resource
    --file-description <STR>  File description resource
    --company <STRING>        Company or author resource
    --copyright <STRING>      Legal copyright resource
    --app-version <VERSION>   Application semantic version
    --identifier <ID>         App bundle / desktop identifier (e.g. com.example.app)
EXAMPLES:
    gpui-lua init my-app
    gpui-lua run
    gpui-lua dev
    gpui-lua run counter.lua -w 800 -h 600 --background acrylic
    gpui-lua build -o dist/app.exe --assets ./assets
"#;

fn main() -> ExitCode {
    // 1. Check if running as a bundled standalone executable
    if let Some(manifest) = check_bundled_app() {
        match LuaApp::from_bundled_manifest(manifest) {
            Ok(app) => {
                if let Err(e) = app.run() {
                    eprintln!("[Error] Application exited with error: {e}");
                    return ExitCode::FAILURE;
                }
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("[Error] Failed to launch bundled application: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    // 2. Parse CLI arguments
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        // If gpui.toml exists in cwd, run it by default
        if AppConfig::find_in(None).is_some() || Path::new("main.lua").is_file() {
            return handle_run(&[], false);
        }
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    match args[0].as_str() {
        "-v" | "--version" | "version" => {
            println!("GPUI.lua 0.1.0 (binary: gpui-lua, Luau JIT, FFmpeg HW-Acceleration, WebRTC)");
            ExitCode::SUCCESS
        }
        "-h" | "--help" | "help" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        "init" => handle_init(&args[1..]),
        "run" => handle_run(&args[1..], false),
        "dev" => handle_run(&args[1..], true),
        "build" => handle_build(&args[1..]),
        script if script.ends_with(".lua") => handle_run(&args, false),
        unknown => {
            eprintln!("[Error] Unknown command or script: {unknown}\n");
            println!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

const GPUI_D_LUA: &str = include_str!("../types/gpui.d.lua");

/// Handler for `gpui.lua init [name]`.
fn handle_init(args: &[String]) -> ExitCode {
    let (target_dir, app_name) = match args.first() {
        Some(name) if name != "." => (PathBuf::from(name), name.clone()),
        _ => (PathBuf::from("."), "My App".to_string()),
    };

    if target_dir != Path::new(".") && !target_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&target_dir) {
            eprintln!("[Error] Failed to create directory {}: {e}", target_dir.display());
            return ExitCode::FAILURE;
        }
    }

    let config_path = target_dir.join("gpui.toml");
    let script_path = target_dir.join("main.lua");
    let d_lua_path = target_dir.join("gpui.d.lua");

    if config_path.exists() {
        println!("[Warning] gpui.toml already exists in {}.", target_dir.display());
    } else {
        let content = AppConfig::starter_template(&app_name, "main.lua");
        if let Err(e) = std::fs::write(&config_path, content) {
            eprintln!("[Error] Failed to write gpui.toml: {e}");
            return ExitCode::FAILURE;
        }
        println!("[Success] Created gpui.toml configuration file.");
    }
    if !script_path.exists() {
        let starter_lua = r##"-- Reactive Counter Application
local count, set_count = signal(0, "counter_val")

function App()
    return ui.div({
        w_full = true,
        h_full = true,
        items_center = true,
        justify_center = true,
        p = 40,
        children = {
            ui.div({
                w = 360,
                p = 24,
                bg = "#313244",
                rounded = 8,
                border = 1,
                border_color = "#313244",
                flex_col = true,
                gap = 12,
                children = {
                    ui.text("GPUI.lua Application"):bold():size(18):color("#cdd6f4"),
                    ui.text("Configured with gpui.toml"):size(13):color("#a6adc8"),
                    ui.div({
                        items_center = true,
                        justify_center = true,
                        py = 16,
                        children = {
                            ui.text(tostring(count())):size(48):bold():color("#b4befe")
                        }
                    }),
                    ui.row({
                        gap = 12,
                        justify = "center",
                        children = {
                            ui.div({
                                bg = "#313244",
                                px = 16, py = 8,
                                rounded = 6,
                                cursor_pointer = true,
                                on_click = function() set_count(count() - 1) end,
                                children = { ui.text("- Decrement"):bold():color("#cdd6f4") }
                            }),
                            ui.div({
                                bg = "#3b82f6",
                                px = 16, py = 8,
                                rounded = 6,
                                cursor_pointer = true,
                                on_click = function() set_count(count() + 1) end,
                                children = { ui.text("+ Increment"):bold():color("#ffffff") }
                            })
                        }
                    })
                }
            })
        }
    })
end
"##;
        if let Err(e) = std::fs::write(&script_path, starter_lua) {
            eprintln!("[Error] Failed to create main.lua: {e}");
            return ExitCode::FAILURE;
        }
        println!("[Success] Created starter main.lua entrypoint.");
    }

    if !d_lua_path.exists() {
        if let Err(e) = std::fs::write(&d_lua_path, GPUI_D_LUA) {
            eprintln!("[Warning] Failed to write gpui.d.lua: {e}");
        } else {
            println!("[Success] Created gpui.d.lua type definitions (LuaLS / EmmyLua).");
        }
    }

    println!("\nReady to run:");
    println!("    gpui-lua dev      # Run with live hot reloading");
    println!("    gpui-lua build    # Bundle into standalone executable");
    ExitCode::SUCCESS
}

struct ParsedCliOptions {
    script_or_entry: Option<PathBuf>,
    config_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    assets_dir: Option<PathBuf>,
    template_path: Option<PathBuf>,
    title: Option<String>,
    width: Option<f32>,
    height: Option<f32>,
    min_width: Option<f32>,
    min_height: Option<f32>,
    resizable: Option<bool>,
    minimizable: Option<bool>,
    csd: Option<bool>,
    csd_height: Option<f32>,
    background: Option<String>,
    windows_background: Option<String>,
    macos_background: Option<String>,
    linux_background: Option<String>,
    product_name: Option<String>,
    file_description: Option<String>,
    company_name: Option<String>,
    copyright: Option<String>,
    icon: Option<String>,
    app_version: Option<String>,
    identifier: Option<String>,
}

fn parse_cli_options(args: &[String]) -> ParsedCliOptions {
    let mut opts = ParsedCliOptions {
        script_or_entry: None,
        config_path: None,
        output_path: None,
        assets_dir: None,
        template_path: None,
        title: None,
        width: None,
        height: None,
        min_width: None,
        min_height: None,
        resizable: None,
        minimizable: None,
        csd: None,
        csd_height: None,
        background: None,
        windows_background: None,
        macos_background: None,
        linux_background: None,
        product_name: None,
        file_description: None,
        company_name: None,
        copyright: None,
        icon: None,
        app_version: None,
        identifier: None,
    };

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if !arg.starts_with('-') && opts.script_or_entry.is_none() {
            opts.script_or_entry = Some(PathBuf::from(arg));
            i += 1;
            continue;
        }

        match arg.as_str() {
            "-c" | "--config" => {
                if i + 1 < args.len() {
                    opts.config_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    opts.output_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "-a" | "--assets" => {
                if i + 1 < args.len() {
                    opts.assets_dir = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--template" => {
                if i + 1 < args.len() {
                    opts.template_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "-t" | "--title" => {
                if i + 1 < args.len() {
                    opts.title = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "-w" | "--width" => {
                if i + 1 < args.len() {
                    opts.width = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            "-h" | "--height" => {
                if i + 1 < args.len() {
                    opts.height = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            "--min-width" => {
                if i + 1 < args.len() {
                    opts.min_width = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            "--min-height" => {
                if i + 1 < args.len() {
                    opts.min_height = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            "--resizable" => opts.resizable = Some(true),
            "--no-resizable" => opts.resizable = Some(false),
            "--minimizable" => opts.minimizable = Some(true),
            "--no-minimizable" => opts.minimizable = Some(false),
            "--csd" => opts.csd = Some(true),
            "--no-csd" => opts.csd = Some(false),
            "--csd-height" => {
                if i + 1 < args.len() {
                    opts.csd_height = args[i + 1].parse().ok();
                    i += 1;
                }
            }
            "--background" => {
                if i + 1 < args.len() {
                    opts.background = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--windows-background" => {
                if i + 1 < args.len() {
                    opts.windows_background = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--macos-background" => {
                if i + 1 < args.len() {
                    opts.macos_background = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--linux-background" => {
                if i + 1 < args.len() {
                    opts.linux_background = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--product-name" => {
                if i + 1 < args.len() {
                    opts.product_name = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--file-description" => {
                if i + 1 < args.len() {
                    opts.file_description = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--company" => {
                if i + 1 < args.len() {
                    opts.company_name = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--copyright" => {
                if i + 1 < args.len() {
                    opts.copyright = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--icon" => {
                if i + 1 < args.len() {
                    opts.icon = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--app-version" => {
                if i + 1 < args.len() {
                    opts.app_version = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--identifier" => {
                if i + 1 < args.len() {
                    opts.identifier = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    opts
}

fn resolve_config_and_entry(
    opts: &ParsedCliOptions,
) -> (AppConfig, PathBuf, Option<PathBuf>) {
    let (mut config, config_file) = if let Some(ref path) = opts.config_path {
        match AppConfig::load_file(path) {
            Ok(c) => (c, Some(path.clone())),
            Err(e) => {
                eprintln!("[Warning] Could not load specified config {}: {e}", path.display());
                (AppConfig::default(), None)
            }
        }
    } else if let Some((c, path)) = AppConfig::find_in(None) {
        (c, Some(path))
    } else {
        (AppConfig::default(), None)
    };

    // Determine entrypoint script
    let entry_script = if let Some(ref p) = opts.script_or_entry {
        p.clone()
    } else if let Some(ref entry) = config.app.entry {
        PathBuf::from(entry)
    } else if Path::new("main.lua").is_file() {
        PathBuf::from("main.lua")
    } else if Path::new("app.lua").is_file() {
        PathBuf::from("app.lua")
    } else {
        PathBuf::from("main.lua")
    };

    // Apply CLI overrides to config struct
    if let Some(ref t) = opts.title {
        config.window.title = Some(t.clone());
    }
    if let Some(w) = opts.width {
        config.window.width = Some(w);
    }
    if let Some(h) = opts.height {
        config.window.height = Some(h);
    }
    if let Some(mw) = opts.min_width {
        config.window.min_width = Some(mw);
    }
    if let Some(mh) = opts.min_height {
        config.window.min_height = Some(mh);
    }
    if let Some(r) = opts.resizable {
        config.window.resizable = Some(r);
    }
    if let Some(m) = opts.minimizable {
        config.window.minimizable = Some(m);
    }
    if let Some(csd) = opts.csd {
        config.window.csd = Some(csd);
    }
    if let Some(ch) = opts.csd_height {
        config.window.csd_height = Some(ch);
    }
    if let Some(ref bg) = opts.background {
        config.window.background = Some(gpui_lua::config::WindowBackgroundConfig::Single(bg.clone()));
    }
    if let Some(ref bg) = opts.windows_background {
        config.window.windows_background = Some(bg.clone());
    }
    if let Some(ref bg) = opts.macos_background {
        config.window.macos_background = Some(bg.clone());
    }
    if let Some(ref bg) = opts.linux_background {
        config.window.linux_background = Some(bg.clone());
    }
    if let Some(ref o) = opts.output_path {
        config.build.output = Some(o.clone());
    }
    if let Some(ref a) = opts.assets_dir {
        config.build.assets = Some(a.clone());
    }
    if let Some(ref tmpl) = opts.template_path {
        config.build.template = Some(tmpl.clone());
    }
    if let Some(ref p) = opts.product_name {
        config.app.product_name = Some(p.clone());
    }
    if let Some(ref d) = opts.file_description {
        config.app.file_description = Some(d.clone());
    }
    if let Some(ref c) = opts.company_name {
        config.app.company_name = Some(c.clone());
    }
    if let Some(ref cr) = opts.copyright {
        config.app.copyright = Some(cr.clone());
    }
    if let Some(ref ic) = opts.icon {
        config.app.icon = Some(ic.clone());
    }
    if let Some(ref v) = opts.app_version {
        config.app.version = Some(v.clone());
    }
    if let Some(ref id) = opts.identifier {
        config.app.identifier = Some(id.clone());
    }
    (config, entry_script, config_file)
}

fn handle_run(args: &[String], hot_reload: bool) -> ExitCode {
    let opts = parse_cli_options(args);
    let (config, script_path, loaded_config) = resolve_config_and_entry(&opts);

    if !script_path.exists() {
        eprintln!("[Error] Script file not found: {}", script_path.display());
        if loaded_config.is_none() {
            eprintln!("Usage: gpui-lua run [script.lua] [options]");
        }
        return ExitCode::FAILURE;
    }

    if let Some(ref path) = loaded_config {
        println!("[Config] Loaded {}", path.display());
    }

    let app = match LuaApp::try_new(&script_path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[Error] Failed to initialize runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Apply configuration and CLI overrides
    let mut app = config.apply_to_app(app);

    let enable_watcher = hot_reload || config.dev.hot_reload.unwrap_or(false);
    if enable_watcher {
        app = app.hot_reload(true);
        println!(
            "[Info] Live hot reload watcher active on {}",
            script_path.display()
        );
    }

    if let Err(e) = app.run() {
        eprintln!("[Error] Application exited with error: {e}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn handle_build(args: &[String]) -> ExitCode {
    let opts = parse_cli_options(args);
    let (config, entry_script, loaded_config) = resolve_config_and_entry(&opts);

    if !entry_script.exists() {
        eprintln!("[Error] Entrypoint script not found: {}", entry_script.display());
        return ExitCode::FAILURE;
    }

    if let Some(ref path) = loaded_config {
        println!("[Config] Loaded {}", path.display());
    }

    // Resolve output destination path
    let default_out_name = format!(
        "{}.exe",
        config
            .app
            .name
            .as_ref()
            .map(|n| n.replace(' ', "_").to_lowercase())
            .unwrap_or_else(|| {
                entry_script
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("app")
                    .to_string()
            })
    );

    let output_path = config
        .build
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(&default_out_name));

    let assets_dir = config.build.assets.clone();

    // Resolve base template executable
    let current_exe = if let Some(ref tmpl) = config.build.template {
        tmpl.clone()
    } else {
        match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[Error] Failed to resolve current executable: {e}");
                return ExitCode::FAILURE;
            }
        }
    };

    println!("[Build] Packaging standalone binary...");
    println!("  Base binary: {}", current_exe.display());
    println!("  Entrypoint:  {}", entry_script.display());
    if let Some(ref a) = assets_dir {
        println!("  Assets dir:  {}", a.display());
    }
    println!("  Destination: {}", output_path.display());

    // 1. Read entrypoint script content
    let entry_name = entry_script
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("main.lua")
        .to_string();

    let entry_bytes = match std::fs::read(&entry_script) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[Error] Failed to read entrypoint script: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut files = HashMap::new();
    files.insert(entry_name.clone(), entry_bytes);

    // 2. Read all files inside assets_dir if supplied
    if let Some(ref assets_path) = assets_dir {
        if assets_path.exists() {
            if let Err(e) = collect_dir_files_public(assets_path, assets_path, &mut files) {
                eprintln!("[Error] Failed to collect assets: {e}");
                return ExitCode::FAILURE;
            }
        } else {
            eprintln!("[Warning] Specified assets directory does not exist: {}", assets_path.display());
        }
    }

    // 3. Construct manifest
    let manifest = AppBundleManifest {
        entrypoint: entry_name,
        title: config.window.title.clone().or_else(|| config.app.resolved_product_name()),
        width: config.window.width,
        height: config.window.height,
        min_width: config.window.min_width,
        min_height: config.window.min_height,
        resizable: config.window.resizable,
        minimizable: config.window.minimizable,
        csd: config.window.csd,
        csd_height: config.window.csd_height,
        background: config.window.active_background(),
        windows_background: config.window.windows_background,
        macos_background: config.window.macos_background,
        linux_background: config.window.linux_background,
        product_name: config.app.resolved_product_name(),
        file_description: config.app.resolved_file_description(),
        company_name: config.app.resolved_company_name(),
        copyright: config.app.copyright,
        icon: config.app.icon,
        identifier: config.app.identifier,
        version: config.app.version,
        files,
    };

    match bundle_package(&current_exe, &output_path, manifest) {
        Ok(()) => {
            let size_bytes = std::fs::metadata(&output_path)
                .map(|m| m.len())
                .unwrap_or(0);
            let size_mb = size_bytes as f64 / (1024.0 * 1024.0);
            println!(
                "\n[Success] Standalone executable created: {} ({:.2} MB)",
                output_path.display(),
                size_mb
            );
            println!("Run directly: {}", output_path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[Error] Build failed: {e}");
            ExitCode::FAILURE
        }
    }
}
