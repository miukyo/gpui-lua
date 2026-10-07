use gpui_lua::{CsdOptions, LuaApp};

fn main() -> anyhow::Result<()> {
    // Initialize LuaApp directly with entrypoint script
    let mut app = LuaApp::new("counter.lua")
        .title("GPUI-CE Rust Application")
        .size(900.0, 650.0)
        .min_size(500.0, 400.0)
        .resizable(true)
        .csd()
        .csd_options(CsdOptions {
            height: 38.0,
            ..Default::default()
        })
        .windows_background(gpui::WindowsWindowBackground::MicaBackdrop);

    // Register native Rust backend capabilities
    app.register_json_fn("rust_greet", |name: String| {
        Ok(format!("Hello from native Rust, {name}!"))
    });

    // Run the GPUI application event loop
    app.run()
}
