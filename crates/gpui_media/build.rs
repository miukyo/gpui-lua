#![allow(clippy::disallowed_methods, reason = "build scripts are exempt")]
fn main() {
    use std::{env, path::PathBuf, process::Command};

    // A build script compiles for the host, so `cfg!(target_os)` here would
    // describe the machine running cargo. The platform being compiled for is
    // only visible through the environment cargo sets for build scripts.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-env-changed=FFMPEG_DIR");
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let embedded_dir = out_dir.join("embedded_dlls");
        let _ = std::fs::create_dir_all(&embedded_dir);

        let required_dlls = [
            "avutil-61.dll",
            "swresample-7.dll",
            "swscale-10.dll",
            "avcodec-63.dll",
            "avformat-63.dll",
        ];

        if let Ok(ffmpeg_dir) = env::var("FFMPEG_DIR") {
            let bin_dir = std::path::Path::new(&ffmpeg_dir).join("bin");
            if bin_dir.exists() {
                for dll_name in &required_dlls {
                    let dll_path = bin_dir.join(dll_name);
                    if dll_path.exists() {
                        if let Ok(raw_bytes) = std::fs::read(&dll_path) {
                            let compressed = miniz_oxide::deflate::compress_to_vec(&raw_bytes, 6);
                            let target_deflate = embedded_dir.join(format!("{dll_name}.deflate"));
                            let _ = std::fs::write(&target_deflate, compressed);
                        }
                    }
                }

                if let Some(target_dir) = out_dir
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent())
                {
                    let examples_dir = target_dir.join("examples");
                    let _ = std::fs::create_dir_all(&examples_dir);
                    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.extension().and_then(|ext| ext.to_str()) == Some("dll") {
                                if let Some(file_name) = path.file_name() {
                                    let _ = std::fs::copy(&path, target_dir.join(file_name));
                                    let _ = std::fs::copy(&path, examples_dir.join(file_name));
                                }
                            }
                        }
                    }
                }
            }
        }
        return;
    }

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    // An explicit SDKROOT (a cross build, or a pinned SDK) wins over asking
    // Xcode, which a non-Apple host does not have.
    println!("cargo:rerun-if-env-changed=SDKROOT");
    let sdk_path = match env::var("SDKROOT") {
        Ok(root) if !root.is_empty() => root,
        _ => String::from_utf8(
            Command::new("xcrun")
                .args(["--sdk", "macosx", "--show-sdk-path"])
                .output()
                .expect("neither SDKROOT nor xcrun can locate the macOS SDK")
                .stdout,
        )
        .unwrap(),
    };
    let sdk_path = sdk_path.trim_end();

    println!("cargo:rerun-if-changed=src/bindings.h");
    let bindings = bindgen::Builder::default()
        .header("src/bindings.h")
        .clang_arg(format!("-isysroot{}", sdk_path))
        .clang_arg("-xobjective-c")
        .allowlist_type("CMItemIndex")
        .allowlist_type("CMSampleTimingInfo")
        .allowlist_type("CMVideoCodecType")
        .allowlist_type("VTEncodeInfoFlags")
        .allowlist_function("CMTimeMake")
        .allowlist_var("kCVPixelFormatType_.*")
        .allowlist_var("kCVReturn.*")
        .allowlist_var("VTEncodeInfoFlags_.*")
        .allowlist_var("kCMVideoCodecType_.*")
        .allowlist_var("kCMTime.*")
        .allowlist_var("kCMSampleAttachmentKey_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false)
        .generate()
        .expect("unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("couldn't write dispatch bindings");
}
