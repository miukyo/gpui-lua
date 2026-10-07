use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Extracts the font family name from raw TrueType or OpenType font bytes.
pub fn extract_font_family_name(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 12 {
        return None;
    }

    let num_tables = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    if bytes.len() < 12 + num_tables * 16 {
        return None;
    }

    let mut name_table_offset = None;
    for i in 0..num_tables {
        let entry = 12 + i * 16;
        let tag = &bytes[entry..entry + 4];
        if tag == b"name" {
            let offset = u32::from_be_bytes([
                bytes[entry + 8],
                bytes[entry + 9],
                bytes[entry + 10],
                bytes[entry + 11],
            ]) as usize;
            name_table_offset = Some(offset);
            break;
        }
    }

    let name_offset = name_table_offset?;
    if name_offset + 6 > bytes.len() {
        return None;
    }

    let name_header = &bytes[name_offset..];
    let count = u16::from_be_bytes([name_header[2], name_header[3]]) as usize;
    let string_offset = name_offset + u16::from_be_bytes([name_header[4], name_header[5]]) as usize;

    if name_offset + 6 + count * 12 > bytes.len() || string_offset > bytes.len() {
        return None;
    }

    let mut preferred_name = None;
    let mut fallback_name = None;

    // name_id 16 = Typographic Family, name_id 1 = Font Family, name_id 4 = Full Name
    for i in 0..count {
        let rec = name_offset + 6 + i * 12;
        let platform_id = u16::from_be_bytes([bytes[rec], bytes[rec + 1]]);
        let encoding_id = u16::from_be_bytes([bytes[rec + 2], bytes[rec + 3]]);
        let name_id = u16::from_be_bytes([bytes[rec + 6], bytes[rec + 7]]);
        let length = u16::from_be_bytes([bytes[rec + 8], bytes[rec + 9]]) as usize;
        let offset = string_offset + u16::from_be_bytes([bytes[rec + 10], bytes[rec + 11]]) as usize;

        if (name_id == 1 || name_id == 16 || name_id == 4) && offset + length <= bytes.len() {
            let str_bytes = &bytes[offset..offset + length];
            let parsed_name = if (platform_id == 3 && (encoding_id == 1 || encoding_id == 10))
                || platform_id == 0
            {
                // UTF-16BE
                let u16s: Vec<u16> = str_bytes
                    .chunks_exact(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16(&u16s).ok()
            } else if platform_id == 1 && encoding_id == 0 {
                // Mac Roman / ASCII
                String::from_utf8(str_bytes.to_vec()).ok()
            } else {
                None
            };

            if let Some(name) = parsed_name {
                let trimmed = name.trim().to_string();
                if !trimmed.is_empty() {
                    if name_id == 16 {
                        preferred_name = Some(trimmed);
                    } else if name_id == 1 {
                        fallback_name = Some(trimmed);
                    } else if fallback_name.is_none() {
                        fallback_name = Some(trimmed);
                    }
                }
            }
        }
    }

    preferred_name.or(fallback_name)
}

/// Helper representing source input for a font.
#[derive(Clone, Debug)]
pub enum FontSource {
    Path(PathBuf),
    Bytes(Cow<'static, [u8]>),
}

impl From<&str> for FontSource {
    fn from(s: &str) -> Self {
        Self::Path(PathBuf::from(s))
    }
}

impl From<String> for FontSource {
    fn from(s: String) -> Self {
        Self::Path(PathBuf::from(s))
    }
}

impl From<&Path> for FontSource {
    fn from(p: &Path) -> Self {
        Self::Path(p.to_path_buf())
    }
}

impl From<PathBuf> for FontSource {
    fn from(p: PathBuf) -> Self {
        Self::Path(p)
    }
}

impl From<&'static [u8]> for FontSource {
    fn from(b: &'static [u8]) -> Self {
        Self::Bytes(Cow::Borrowed(b))
    }
}

impl From<Vec<u8>> for FontSource {
    fn from(b: Vec<u8>) -> Self {
        Self::Bytes(Cow::Owned(b))
    }
}

impl From<Cow<'static, [u8]>> for FontSource {
    fn from(b: Cow<'static, [u8]>) -> Self {
        Self::Bytes(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_font_family_mock() {
        // Build mock OpenType table directory with 'name' table
        let mut data = Vec::new();
        // sfntVersion = 0x00010000, numTables = 1
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // searchRange, entrySelector, rangeShift

        // Table entry for 'name' at offset 28
        data.extend_from_slice(b"name");
        data.extend_from_slice(&0u32.to_be_bytes()); // checkSum
        data.extend_from_slice(&28u32.to_be_bytes()); // offset = 28
        data.extend_from_slice(&100u32.to_be_bytes()); // length

        // Name table at offset 28
        // format = 0, count = 1, stringOffset = 18 (from start of name table, so offset 28 + 18 = 46)
        data.extend_from_slice(&0u16.to_be_bytes());
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(&18u16.to_be_bytes());

        // Name record 0: platform 3 (Windows), encoding 1 (Unicode), lang 0x0409, name_id 1 (Family), length 10, offset 0
        data.extend_from_slice(&3u16.to_be_bytes());
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(&0x0409u16.to_be_bytes());
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(&10u16.to_be_bytes()); // "Inter" = 5 chars * 2 bytes = 10
        data.extend_from_slice(&0u16.to_be_bytes());

        // String storage at offset 46: "Inter" in UTF-16BE
        for c in "Inter".encode_utf16() {
            data.extend_from_slice(&c.to_be_bytes());
        }

        let family = extract_font_family_name(&data);
        assert_eq!(family.as_deref(), Some("Inter"));
    }
}
