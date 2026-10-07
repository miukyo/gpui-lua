use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
pub mod resources;

pub const BUNDLE_MAGIC: &[u8; 8] = b"GPUILUAP";
pub const TRAILER_SIZE: u64 = 24; // 8 bytes offset + 8 bytes len + 8 bytes magic
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppBundleManifest {
    pub entrypoint: String,
    pub title: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
    pub resizable: Option<bool>,
    pub minimizable: Option<bool>,
    pub csd: Option<bool>,
    pub csd_height: Option<f32>,
    pub background: Option<String>,
    pub windows_background: Option<String>,
    pub macos_background: Option<String>,
    pub linux_background: Option<String>,
    pub files: HashMap<String, Vec<u8>>,

    // Executable Resources & OS Metadata
    pub version: Option<String>,
    pub product_name: Option<String>,
    pub file_description: Option<String>,
    pub company_name: Option<String>,
    pub copyright: Option<String>,
    pub icon: Option<String>,
    pub identifier: Option<String>,
}

impl Default for AppBundleManifest {
    fn default() -> Self {
        Self {
            entrypoint: "main.lua".to_string(),
            title: Some("GPUI Application".to_string()),
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
            files: HashMap::new(),
            version: Some("0.1.0".to_string()),
            product_name: None,
            file_description: None,
            company_name: None,
            copyright: None,
            icon: None,
            identifier: None,
        }
    }
}

/// Checks if the currently running executable has a bundled Lua application package appended to it.
pub fn check_bundled_app() -> Option<AppBundleManifest> {
    let current_exe = std::env::current_exe().ok()?;
    read_bundled_manifest(&current_exe).ok()
}

/// Reads bundled package from the given executable file if present.
pub fn read_bundled_manifest(exe_path: &Path) -> Result<AppBundleManifest> {
    let mut file = File::open(exe_path).with_context(|| format!("Failed to open {}", exe_path.display()))?;
    let file_len = file.metadata()?.len();

    if file_len < TRAILER_SIZE {
        anyhow::bail!("File is smaller than trailer size");
    }

    // Read trailer at end of file
    file.seek(SeekFrom::End(-(TRAILER_SIZE as i64)))?;
    let mut trailer = [0u8; 24];
    file.read_exact(&mut trailer)?;

    let magic = &trailer[16..24];
    if magic != BUNDLE_MAGIC {
        anyhow::bail!("No bundle magic signature found");
    }

    let data_offset = u64::from_le_bytes(trailer[0..8].try_into().unwrap());
    let data_len = u64::from_le_bytes(trailer[8..16].try_into().unwrap());

    if data_offset + data_len + TRAILER_SIZE > file_len {
        anyhow::bail!("Corrupted bundle trailer offsets");
    }

    file.seek(SeekFrom::Start(data_offset))?;
    let mut compressed_data = vec![0u8; data_len as usize];
    file.read_exact(&mut compressed_data)?;

    let decompressed = miniz_oxide::inflate::decompress_to_vec(&compressed_data)
        .map_err(|e| anyhow::anyhow!("Decompression failed: {e:?}"))?;

    let manifest: AppBundleManifest = serde_json::from_slice(&decompressed)
        .context("Failed to deserialize bundle manifest")?;

    Ok(manifest)
}

/// Packages an application using a fully populated `AppBundleManifest`.
pub fn bundle_package(
    base_exe_path: &Path,
    output_path: &Path,
    manifest: AppBundleManifest,
) -> Result<()> {
    // 1. Serialize and compress manifest
    let manifest_json = serde_json::to_vec(&manifest)?;
    let compressed_payload = miniz_oxide::deflate::compress_to_vec(&manifest_json, 6);

    // 2. Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    // 3. Copy base executable to output destination
    std::fs::copy(base_exe_path, output_path)
        .with_context(|| format!("Failed to copy base executable from {} to {}", base_exe_path.display(), output_path.display()))?;

    // 4. Append compressed payload and 24-byte trailer to output binary
    let mut out_file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(output_path)
        .with_context(|| format!("Failed to open output binary for appending: {}", output_path.display()))?;

    let data_offset = out_file.metadata()?.len();
    let data_len = compressed_payload.len() as u64;

    out_file.write_all(&compressed_payload)?;

    let mut trailer = [0u8; 24];
    trailer[0..8].copy_from_slice(&data_offset.to_le_bytes());
    trailer[8..16].copy_from_slice(&data_len.to_le_bytes());
    trailer[16..24].copy_from_slice(BUNDLE_MAGIC);

    out_file.write_all(&trailer)?;
    out_file.flush()?;
    drop(out_file);

    // 5. Apply OS-specific executable resources (PE version/icon, macOS .app Info.plist, Linux .desktop)
    resources::apply_executable_resources(output_path, &manifest)?;

    Ok(())
}

/// Packages a Lua script and optional assets directory into a standalone single executable binary.
pub fn bundle_standalone_binary(
    base_exe_path: &Path,
    entrypoint_script: &Path,
    assets_dir: Option<&Path>,
    output_path: &Path,
    title: Option<String>,
    width: Option<f32>,
    height: Option<f32>,
    csd: bool,
) -> Result<()> {
    // 1. Read entrypoint script content
    let entry_name = entrypoint_script
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("main.lua")
        .to_string();

    let entry_bytes = std::fs::read(entrypoint_script)
        .with_context(|| format!("Failed to read entrypoint script: {}", entrypoint_script.display()))?;

    let mut files = HashMap::new();
    files.insert(entry_name.clone(), entry_bytes);

    // 2. Read all files inside assets_dir recursively if supplied
    if let Some(assets_path) = assets_dir {
        if assets_path.exists() {
            collect_dir_files(assets_path, assets_path, &mut files)?;
        }
    }

    let manifest = AppBundleManifest {
        entrypoint: entry_name,
        title,
        width,
        height,
        min_width: Some(300.0),
        min_height: Some(200.0),
        resizable: Some(true),
        minimizable: Some(true),
        csd: Some(csd),
        csd_height: Some(38.0),
        background: None,
        windows_background: None,
        macos_background: None,
        linux_background: None,
        files,
        version: Some("0.1.0".to_string()),
        product_name: None,
        file_description: None,
        company_name: None,
        copyright: None,
        icon: None,
        identifier: None,
    };

    bundle_package(base_exe_path, output_path, manifest)
}

#[doc(hidden)]
pub fn collect_dir_files_public(root: &Path, current: &Path, map: &mut HashMap<String, Vec<u8>>) -> Result<()> {
    collect_dir_files(root, current, map)
}
fn collect_dir_files(root: &Path, current: &Path, map: &mut HashMap<String, Vec<u8>>) -> Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_dir_files(root, &path, map)?;
        } else if path.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                let key = rel.to_string_lossy().replace('\\', "/");
                let data = std::fs::read(&path)?;
                map.insert(key, data);
            }
        }
    }
    Ok(())
}
