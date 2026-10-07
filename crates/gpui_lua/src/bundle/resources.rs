use anyhow::Result;
use std::path::Path;
use crate::bundle::AppBundleManifest;

#[cfg(target_os = "windows")]
use windows::core::PCWSTR;
#[cfg(target_os = "windows")]
use windows::Win32::System::LibraryLoader::{BeginUpdateResourceW, EndUpdateResourceW, UpdateResourceW};

/// Applies platform-specific executable resources (Windows version info / icon, macOS .app bundle Info.plist, Linux .desktop).
pub fn apply_executable_resources(output_path: &Path, manifest: &AppBundleManifest) -> Result<()> {
    #[cfg(target_os = "windows")]
    if let Err(e) = apply_windows_pe_resources(output_path, manifest) {
        log::warn!("Could not apply Windows PE resources: {e}");
    }

    #[cfg(target_os = "macos")]
    if let Err(e) = apply_macos_resources(output_path, manifest) {
        log::warn!("Could not apply macOS app bundle resources: {e}");
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    if let Err(e) = apply_linux_resources(output_path, manifest) {
        log::warn!("Could not apply Linux desktop resources: {e}");
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn apply_windows_pe_resources(exe_path: &Path, manifest: &AppBundleManifest) -> Result<()> {
    if !exe_path.exists() {
        return Ok(());
    }

    // Convert path to wide string
    let path_str = exe_path.to_string_lossy().to_string();
    let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let handle = match BeginUpdateResourceW(PCWSTR(wide_path.as_ptr()), false) {
            Ok(h) if !h.is_invalid() => h,
            _ => return Ok(()),
        };
        // 1. Version info resource (RT_VERSION = 16, MAKEINTRESOURCE(1) = 1)
        if let Some(version_data) = build_vs_version_info(manifest, exe_path) {
            let rt_version = 16 as usize as *const u16;
            let res_id = 1 as usize as *const u16;
            let _ = UpdateResourceW(
                handle,
                PCWSTR(rt_version),
                PCWSTR(res_id),
                0x0409, // US English
                Some(version_data.as_ptr() as *const _),
                version_data.len() as u32,
            );
        }

        // 2. Icon resource (RT_ICON = 3, RT_GROUP_ICON = 14)
        if let Some(icon_path_str) = &manifest.icon {
            let icon_bytes = if let Some(bytes) = manifest.files.get(icon_path_str) {
                Some(bytes.clone())
            } else if let Ok(bytes) = std::fs::read(icon_path_str) {
                Some(bytes)
            } else {
                None
            };

            if let Some(bytes) = icon_bytes {
                let _ = apply_ico_resources(handle, &bytes);
            }
        }

        let _ = EndUpdateResourceW(handle, false);
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_ico_resources(
    handle: windows::Win32::Foundation::HANDLE,
    ico_bytes: &[u8],
) -> Result<()> {
    if ico_bytes.len() < 6 {
        return Ok(());
    }

    let id_type = u16::from_le_bytes([ico_bytes[2], ico_bytes[3]]);
    let count = u16::from_le_bytes([ico_bytes[4], ico_bytes[5]]) as usize;

    if id_type != 1 || count == 0 {
        return Ok(());
    }

    let mut grp_icon_dir = Vec::new();
    grp_icon_dir.extend_from_slice(&ico_bytes[0..6]); // idReserved, idType, idCount

    let rt_icon = 3 as usize as *const u16;
    let rt_group_icon = 14 as usize as *const u16;

    for i in 0..count {
        let entry_offset = 6 + i * 16;
        if entry_offset + 16 > ico_bytes.len() {
            break;
        }

        let entry = &ico_bytes[entry_offset..entry_offset + 16];
        let bytes_in_res = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]) as usize;
        let image_offset = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]) as usize;

        if image_offset + bytes_in_res > ico_bytes.len() {
            continue;
        }

        let image_data = &ico_bytes[image_offset..image_offset + bytes_in_res];
        let icon_id = (i + 1) as u16;

        unsafe {
            let res_id = icon_id as usize as *const u16;
            let _ = UpdateResourceW(
                handle,
                PCWSTR(rt_icon),
                PCWSTR(res_id),
                0x0409,
                Some(image_data.as_ptr() as *const _),
                image_data.len() as u32,
            );
        }

        // Add to GRPICONDIR: 12 bytes of directory entry + 2 bytes nID
        grp_icon_dir.extend_from_slice(&entry[0..12]);
        grp_icon_dir.extend_from_slice(&icon_id.to_le_bytes());
    }

    unsafe {
        let group_id = 1 as usize as *const u16;
        let _ = UpdateResourceW(
            handle,
            PCWSTR(rt_group_icon),
            PCWSTR(group_id),
            0x0409,
            Some(grp_icon_dir.as_ptr() as *const _),
            grp_icon_dir.len() as u32,
        );
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn build_vs_version_info(manifest: &AppBundleManifest, exe_path: &Path) -> Option<Vec<u8>> {
    let version_str = manifest.version.as_deref().unwrap_or("1.0.0.0");
    let file_desc = manifest.file_description.as_deref()
        .or(manifest.title.as_deref())
        .unwrap_or("GPUI Application");
    let product_name = manifest.product_name.as_deref()
        .or(manifest.title.as_deref())
        .unwrap_or("GPUI Application");
    let company_name = manifest.company_name.as_deref().unwrap_or("");
    let copyright = manifest.copyright.as_deref().unwrap_or("");
    let original_filename = exe_path.file_name().and_then(|n| n.to_str()).unwrap_or("app.exe");

    let version_parts: Vec<u16> = version_str
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let v_major = version_parts.get(0).copied().unwrap_or(1);
    let v_minor = version_parts.get(1).copied().unwrap_or(0);
    let v_patch = version_parts.get(2).copied().unwrap_or(0);
    let v_build = version_parts.get(3).copied().unwrap_or(0);

    let mut strings = Vec::new();
    strings.push(("FileDescription", file_desc));
    strings.push(("ProductName", product_name));
    strings.push(("FileVersion", version_str));
    strings.push(("ProductVersion", version_str));
    strings.push(("OriginalFilename", original_filename));
    if !company_name.is_empty() {
        strings.push(("CompanyName", company_name));
    }
    if !copyright.is_empty() {
        strings.push(("LegalCopyright", copyright));
    }

    let mut buf = Vec::new();

    // Helper to pad to 32-bit (4-byte) boundary
    fn pad4(b: &mut Vec<u8>) {
        while b.len() % 4 != 0 {
            b.push(0);
        }
    }

    fn write_utf16(b: &mut Vec<u8>, s: &str) {
        for c in s.encode_utf16() {
            b.extend_from_slice(&c.to_le_bytes());
        }
        b.extend_from_slice(&0u16.to_le_bytes()); // null terminator
    }

    // 1. Build StringTable children
    let mut string_table_bytes = Vec::new();
    for (k, val) in strings {
        let entry_start = string_table_bytes.len();
        string_table_bytes.extend_from_slice(&0u16.to_le_bytes()); // wLength placeholder
        let val_wlen = (val.encode_utf16().count() + 1) as u16;
        string_table_bytes.extend_from_slice(&val_wlen.to_le_bytes()); // wValueLength (in WCHARs)
        string_table_bytes.extend_from_slice(&1u16.to_le_bytes()); // wType = 1 (text)
        write_utf16(&mut string_table_bytes, k);
        pad4(&mut string_table_bytes);
        write_utf16(&mut string_table_bytes, val);
        pad4(&mut string_table_bytes);
        let entry_len = (string_table_bytes.len() - entry_start) as u16;
        string_table_bytes[entry_start..entry_start + 2].copy_from_slice(&entry_len.to_le_bytes());
    }

    // 2. Build StringTable header ("040904B0")
    let mut string_file_info_bytes = Vec::new();
    let st_start = string_file_info_bytes.len();
    string_file_info_bytes.extend_from_slice(&0u16.to_le_bytes()); // wLength placeholder
    string_file_info_bytes.extend_from_slice(&0u16.to_le_bytes()); // wValueLength = 0
    string_file_info_bytes.extend_from_slice(&1u16.to_le_bytes()); // wType = 1
    write_utf16(&mut string_file_info_bytes, "040904B0");
    pad4(&mut string_file_info_bytes);
    string_file_info_bytes.extend_from_slice(&string_table_bytes);
    let st_len = (string_file_info_bytes.len() - st_start) as u16;
    string_file_info_bytes[st_start..st_start + 2].copy_from_slice(&st_len.to_le_bytes());

    // 3. Wrap in StringFileInfo
    let mut sfi_wrapper = Vec::new();
    let sfi_start = sfi_wrapper.len();
    sfi_wrapper.extend_from_slice(&0u16.to_le_bytes()); // wLength
    sfi_wrapper.extend_from_slice(&0u16.to_le_bytes()); // wValueLength = 0
    sfi_wrapper.extend_from_slice(&1u16.to_le_bytes()); // wType = 1
    write_utf16(&mut sfi_wrapper, "StringFileInfo");
    pad4(&mut sfi_wrapper);
    sfi_wrapper.extend_from_slice(&string_file_info_bytes);
    let sfi_len = (sfi_wrapper.len() - sfi_start) as u16;
    sfi_wrapper[sfi_start..sfi_start + 2].copy_from_slice(&sfi_len.to_le_bytes());

    // 4. Build VarFileInfo
    let mut var_file_info = Vec::new();
    let vfi_start = var_file_info.len();
    var_file_info.extend_from_slice(&0u16.to_le_bytes());
    var_file_info.extend_from_slice(&0u16.to_le_bytes());
    var_file_info.extend_from_slice(&1u16.to_le_bytes());
    write_utf16(&mut var_file_info, "VarFileInfo");
    pad4(&mut var_file_info);

    let var_start = var_file_info.len();
    var_file_info.extend_from_slice(&0u16.to_le_bytes());
    var_file_info.extend_from_slice(&4u16.to_le_bytes()); // wValueLength = 4 bytes
    var_file_info.extend_from_slice(&0u16.to_le_bytes()); // wType = 0 (binary)
    write_utf16(&mut var_file_info, "Translation");
    pad4(&mut var_file_info);
    var_file_info.extend_from_slice(&0x0409u16.to_le_bytes()); // language = US English
    var_file_info.extend_from_slice(&0x04B0u16.to_le_bytes()); // codepage = Unicode
    let var_len = (var_file_info.len() - var_start) as u16;
    var_file_info[var_start..var_start + 2].copy_from_slice(&var_len.to_le_bytes());

    let vfi_len = (var_file_info.len() - vfi_start) as u16;
    var_file_info[vfi_start..vfi_start + 2].copy_from_slice(&vfi_len.to_le_bytes());

    // 5. Construct VS_FIXEDFILEINFO (52 bytes)
    let mut fixed_info = [0u8; 52];
    fixed_info[0..4].copy_from_slice(&0xFEEF04BDu32.to_le_bytes()); // dwSignature
    fixed_info[4..8].copy_from_slice(&0x00010000u32.to_le_bytes()); // dwStrucVersion (1.0)
    let file_ver_ms = ((v_major as u32) << 16) | (v_minor as u32);
    let file_ver_ls = ((v_patch as u32) << 16) | (v_build as u32);
    fixed_info[8..12].copy_from_slice(&file_ver_ms.to_le_bytes()); // dwFileVersionMS
    fixed_info[12..16].copy_from_slice(&file_ver_ls.to_le_bytes()); // dwFileVersionLS
    fixed_info[16..20].copy_from_slice(&file_ver_ms.to_le_bytes()); // dwProductVersionMS
    fixed_info[20..24].copy_from_slice(&file_ver_ls.to_le_bytes()); // dwProductVersionLS
    fixed_info[24..28].copy_from_slice(&0x0000003Fu32.to_le_bytes()); // dwFileFlagsMask
    fixed_info[28..32].copy_from_slice(&0x00000000u32.to_le_bytes()); // dwFileFlags
    fixed_info[32..36].copy_from_slice(&0x00040004u32.to_le_bytes()); // dwFileOS = VOS_NT_WINDOWS32
    fixed_info[36..40].copy_from_slice(&0x00000001u32.to_le_bytes()); // dwFileType = VFT_APP
    fixed_info[40..44].copy_from_slice(&0x00000000u32.to_le_bytes()); // dwFileSubtype

    // 6. Assemble complete VS_VERSION_INFO root
    buf.extend_from_slice(&0u16.to_le_bytes()); // total wLength placeholder
    buf.extend_from_slice(&(fixed_info.len() as u16).to_le_bytes()); // wValueLength = 52
    buf.extend_from_slice(&0u16.to_le_bytes()); // wType = 0 (binary)
    write_utf16(&mut buf, "VS_VERSION_INFO");
    pad4(&mut buf);
    buf.extend_from_slice(&fixed_info);
    pad4(&mut buf);
    buf.extend_from_slice(&sfi_wrapper);
    pad4(&mut buf);
    buf.extend_from_slice(&var_file_info);

    let total_len = buf.len() as u16;
    buf[0..2].copy_from_slice(&total_len.to_le_bytes());

    Some(buf)
}

/// Generates macOS .app bundle directory structure and Info.plist.
pub fn apply_macos_resources(output_path: &Path, manifest: &AppBundleManifest) -> Result<()> {
    let app_name = manifest.product_name.as_deref()
        .or(manifest.title.as_deref())
        .unwrap_or("App");
    let version = manifest.version.as_deref().unwrap_or("1.0.0");
    let desc = manifest.file_description.as_deref().unwrap_or(app_name);
    let identifier = manifest.identifier.as_deref().unwrap_or("com.gpui.app");
    let copyright = manifest.copyright.as_deref().unwrap_or("");
    let icon_name = manifest.icon.as_deref().and_then(|p| Path::new(p).file_name()?.to_str()).unwrap_or("AppIcon.icns");

    // If output is App.app, generate bundle structure
    let bundle_dir = if output_path.extension().and_then(|e| e.to_str()) == Some("app") {
        output_path.to_path_buf()
    } else {
        output_path.with_extension("app")
    };

    let contents_dir = bundle_dir.join("Contents");
    let macos_dir = contents_dir.join("MacOS");
    let resources_dir = contents_dir.join("Resources");

    std::fs::create_dir_all(&macos_dir)?;
    std::fs::create_dir_all(&resources_dir)?;

    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleName</key>
    <string>{app_name}</string>
    <key>CFBundleDisplayName</key>
    <string>{desc}</string>
    <key>CFBundleIdentifier</key>
    <string>{identifier}</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundleExecutable</key>
    <string>{app_name}</string>
    <key>CFBundleIconFile</key>
    <string>{icon_name}</string>
    <key>NSHumanReadableCopyright</key>
    <string>{copyright}</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
"#
    );

    std::fs::write(contents_dir.join("Info.plist"), plist_content)?;

    // If icon file provided, copy to Resources
    if let Some(icon_path) = &manifest.icon {
        if let Some(bytes) = manifest.files.get(icon_path) {
            let _ = std::fs::write(resources_dir.join(icon_name), bytes);
        } else if let Ok(bytes) = std::fs::read(icon_path) {
            let _ = std::fs::write(resources_dir.join(icon_name), bytes);
        }
    }

    Ok(())
}

/// Generates Linux Freedesktop .desktop entry file.
pub fn apply_linux_resources(output_path: &Path, manifest: &AppBundleManifest) -> Result<()> {
    let app_name = manifest.product_name.as_deref()
        .or(manifest.title.as_deref())
        .unwrap_or("App");
    let version = manifest.version.as_deref().unwrap_or("1.0.0");
    let desc = manifest.file_description.as_deref().unwrap_or(app_name);
    let icon = manifest.icon.as_deref().unwrap_or("");
    let bin_name = output_path.file_name().and_then(|n| n.to_str()).unwrap_or("app");

    let desktop_content = format!(
        r#"[Desktop Entry]
Type=Application
Name={app_name}
Comment={desc}
Version={version}
Exec={bin_name}
Icon={icon}
Terminal=false
Categories=Utility;Application;
"#
    );

    let desktop_path = output_path.with_extension("desktop");
    std::fs::write(desktop_path, desktop_content)?;

    Ok(())
}
