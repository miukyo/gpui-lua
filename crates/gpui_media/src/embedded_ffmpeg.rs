//! Embedded FFmpeg dynamic library extractor for Windows.
//! Automatically unpacks compressed FFmpeg DLLs to LocalAppData if not present in the system,
//! ensuring self-contained single-binary distribution with MSVC /DELAYLOAD.

use std::path::PathBuf;
use std::sync::Once;

const AVUTIL_DEFLATE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/embedded_dlls/avutil-61.dll.deflate"));
const SWRESAMPLE_DEFLATE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/embedded_dlls/swresample-7.dll.deflate"));
const SWSCALE_DEFLATE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/embedded_dlls/swscale-10.dll.deflate"));
const AVCODEC_DEFLATE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/embedded_dlls/avcodec-63.dll.deflate"));
const AVFORMAT_DEFLATE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/embedded_dlls/avformat-63.dll.deflate"));

static EMBEDDED_DLLS: &[(&str, &[u8])] = &[
    ("avutil-61.dll", AVUTIL_DEFLATE),
    ("swresample-7.dll", SWRESAMPLE_DEFLATE),
    ("swscale-10.dll", SWSCALE_DEFLATE),
    ("avcodec-63.dll", AVCODEC_DEFLATE),
    ("avformat-63.dll", AVFORMAT_DEFLATE),
];

static INIT_ONCE: Once = Once::new();

#[ctor::ctor(unsafe)]
fn auto_init() {
    ensure_ffmpeg_dlls();
}

pub fn ensure_ffmpeg_dlls() {
    INIT_ONCE.call_once(|| {
        // 1. If avcodec-63.dll is already loaded in process, nothing to do
        unsafe {
            use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;
            let test_mod = GetModuleHandleA(b"avcodec-63.dll\0".as_ptr());
            if test_mod != std::ptr::null_mut() {
                return;
            }
        }

        // If loose DLL exists next to current executable, allow it
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                if exe_dir.join("avcodec-63.dll").exists() {
                    return;
                }
            }
        }

        // 2. Resolve destination directory in LocalAppData or Temp
        let base_dir = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);

        let target_dir = base_dir.join("gpui-ce").join("ffmpeg-63");
        let _ = std::fs::create_dir_all(&target_dir);

        // 3. Extract missing DLLs
        for (dll_name, deflated) in EMBEDDED_DLLS {
            let out_file = target_dir.join(dll_name);
            if !out_file.exists() {
                if let Ok(raw_data) = miniz_oxide::inflate::decompress_to_vec(deflated) {
                    let _ = std::fs::write(&out_file, raw_data);
                }
            }
        }

        // 4. Register target_dir with SetDllDirectoryW so delay-load helper resolves them
        use std::os::windows::ffi::OsStrExt;
        let mut wide: Vec<u16> = target_dir.as_os_str().encode_wide().collect();
        wide.push(0);
        unsafe {
            windows_sys::Win32::System::LibraryLoader::SetDllDirectoryW(wide.as_ptr());
        }
    });
}
