//! The name the phone's owner gave it, as the installed system's settings
//! keep it: MIUI writes the name chosen under 我的设备 → 设备名称 there,
//! where the build properties only name the model.

use std::collections::HashMap;

/// Where the owner's secure settings are kept, in the data partition
/// mounted at /data: only there once the partition is open.
const SETTINGS: &str = "/data/system/users/0/settings_secure.xml";

/// The settings the name is kept in, in the order they are looked in: MIUI
/// writes the name the owner chose to `bluetooth_name`, and `device_name`
/// is Android's own. The global `device_name` is not read: on MIUI it keeps
/// the model name, not the owner's.
const KEYS: [&str; 2] = ["bluetooth_name", "device_name"];

/// A setting's name and what it is set to.
type Settings = HashMap<String, String>;

/// The name the owner gave the phone, if its settings are readable.
pub(crate) fn owner_name() -> Option<String> {
    let bytes = std::fs::read(SETTINGS).ok()?;
    let names = if binary(&bytes) { axml(&bytes)? } else { text(&String::from_utf8_lossy(&bytes)) };
    name_of(&names)
}

/// Whether `bytes` are Android's binary XML, which starts with the chunk
/// header of an XML document, rather than the text of a document.
fn binary(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x03, 0x00, 0x08, 0x00])
}

/// The name among `names`, the first of [`KEYS`] one is under.
fn name_of(names: &HashMap<String, String>) -> Option<String> {
    KEYS.iter().find_map(|key| names.get(*key).cloned()).filter(|name| !name.trim().is_empty())
}

/// The settings a text document writes, by name.
fn text(xml: &str) -> HashMap<String, String> {
    let mut names = HashMap::new();
    for tag in xml.split('<').skip(1) {
        let end = tag.find(|c: char| c.is_whitespace() || c == '/' || c == '>').unwrap_or(tag.len());
        if &tag[..end] != "setting" {
            continue;
        }
        let (Some(key), Some(value)) = (attribute(tag, "name"), attribute(tag, "value")) else { continue };
        names.insert(unescape(key), unescape(value));
    }
    names
}

/// The text `name` is written as an attribute of `tag` with: a name of its
/// own, not the end of a longer one such as `bluetooth_name`.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut from = 0;
    while let Some(found) = tag[from..].find(name) {
        let at = from + found;
        if tag[..at].chars().next_back().is_some_and(char::is_whitespace)
            && let Some(rest) = tag[at + name.len()..].strip_prefix("=\"")
        {
            return Some(&rest[..rest.find('"')?]);
        }
        from = at + name.len();
    }
    None
}

/// `text` with the character references a settings file writes undone.
fn unescape(text: &str) -> String {
    text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// The settings a binary document writes, by name: its string pool, then
/// each `setting` element's `name` and `value` attributes.
fn axml(bytes: &[u8]) -> Option<HashMap<String, String>> {
    if u16_at(bytes, 0)? != 0x0003 {
        return None;
    }
    let end = (u32_at(bytes, 4)? as usize).min(bytes.len());
    let mut strings: Vec<String> = Vec::new();
    let mut names = HashMap::new();
    let mut at = u16_at(bytes, 2)? as usize;
    while at + 8 <= end {
        let kind = u16_at(bytes, at)?;
        let header = u16_at(bytes, at + 2)? as usize;
        let size = u32_at(bytes, at + 4)? as usize;
        if size < 8 || at + size > end {
            break;
        }
        let chunk = &bytes[at..at + size];
        match kind {
            0x0001 => strings = pool(chunk)?,
            0x0102 if header >= 16 => {
                if let Some((element, attributes)) = element(chunk, &strings)
                    && element == "setting"
                    && let (Some(key), Some(value)) = (attributes.get("name"), attributes.get("value"))
                {
                    names.insert(key.clone(), value.clone());
                }
            }
            _ => {}
        }
        at += size;
    }
    Some(names)
}

/// The strings of a string pool chunk, in the order a document names them.
fn pool(chunk: &[u8]) -> Option<Vec<String>> {
    let count = u32_at(chunk, 8)? as usize;
    let flags = u32_at(chunk, 16)?;
    let start = u32_at(chunk, 20)? as usize;
    (0..count)
        .map(|i| {
            let offset = u32_at(chunk, 28 + i * 4)? as usize;
            let string = chunk.get(start + offset..)?;
            if flags & 0x0100 != 0 { utf8(string) } else { utf16(string) }
        })
        .collect()
}

/// A string length as a variable-length 8-bit number: one byte, or two
/// where the first has its top bit set.
fn length8(bytes: &[u8], at: usize) -> Option<(usize, usize)> {
    let first = *bytes.get(at)? as usize;
    if first & 0x80 == 0 {
        Some((first, at + 1))
    } else {
        Some((((first & 0x7f) << 8) | *bytes.get(at + 1)? as usize, at + 2))
    }
}

/// A string of a UTF-8 string pool: its length in characters, then in
/// bytes, which is how many follow.
fn utf8(bytes: &[u8]) -> Option<String> {
    let (_, at) = length8(bytes, 0)?;
    let (length, at) = length8(bytes, at)?;
    Some(String::from_utf8_lossy(bytes.get(at..at + length)?).into_owned())
}

/// A string of a UTF-16 string pool, its length a variable-length 16-bit
/// number: one unit, or two where the first has its top bit set.
fn utf16(bytes: &[u8]) -> Option<String> {
    let first = u16_at(bytes, 0)? as usize;
    let (length, at) = if first & 0x8000 == 0 { (first, 2) } else { (((first & 0x7fff) << 16) | u16_at(bytes, 2)? as usize, 4) };
    let units: Vec<u16> = (0..length).map(|i| u16_at(bytes, at + i * 2)).collect::<Option<_>>()?;
    Some(String::from_utf16_lossy(&units))
}

/// An element chunk's name and attributes, by the strings its indexes name.
fn element(chunk: &[u8], strings: &[String]) -> Option<(String, Settings)> {
    let name = strings.get(u32_at(chunk, 20)? as usize)?.clone();
    let start = u16_at(chunk, 24)? as usize;
    let size = u16_at(chunk, 26)? as usize;
    let count = u16_at(chunk, 28)? as usize;
    let mut attributes = HashMap::with_capacity(count);
    for i in 0..count {
        let at = 16 + start + i * size;
        let key = strings.get(u32_at(chunk, at + 4)? as usize)?.clone();
        let raw = u32_at(chunk, at + 8)?;
        let kind = *chunk.get(at + 15)?;
        let value = if raw != u32::MAX {
            strings.get(raw as usize)?.clone()
        } else if kind == 0x03 {
            strings.get(u32_at(chunk, at + 16)? as usize)?.clone()
        } else {
            continue;
        };
        attributes.insert(key, value);
    }
    Some((name, attributes))
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let bytes = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_is_read_from_a_text_settings_document() {
        let xml = "<?xml version='1.0' encoding='utf-8' standalone='yes' ?>\n\
<settings version=\"-1\">\n\
<setting id=\"1\" name=\"device_name\" value=\"旧名字\" package=\"com.android.settings\" />\n\
<setting id=\"2\" name=\"bluetooth_name\" value=\"碎屏飞牛\" package=\"com.android.settings\" />\n\
</settings>\n";
        let names = text(xml);
        assert_eq!(name_of(&names).as_deref(), Some("碎屏飞牛"), "bluetooth_name comes first");
    }

    #[test]
    fn the_name_is_read_from_a_binary_settings_document() {
        for utf8_pool in [true, false] {
            let names = axml(&binary_settings("bluetooth_name", "碎屏飞牛", utf8_pool)).unwrap();
            assert_eq!(name_of(&names).as_deref(), Some("碎屏飞牛"), "a {} string pool", if utf8_pool { "UTF-8" } else { "UTF-16" });
        }
    }

    #[test]
    fn a_document_without_the_owners_name_names_none() {
        let names = text("<settings><setting name=\"device_name\" value=\" \" /></settings>");
        assert_eq!(name_of(&names), None, "an empty name is none");
        assert_eq!(name_of(&text("<settings />")), None);
    }

    /// A binary document holding one `setting` element with `key` and
    /// `value`, its string pool written UTF-8 or UTF-16.
    fn binary_settings(key: &str, value: &str, utf8_pool: bool) -> Vec<u8> {
        let strings = ["setting", "name", "value", key, value];
        let mut document = vec![0x03, 0x00, 8, 0x00, 0, 0, 0, 0];
        document.extend(pool_of(&strings, utf8_pool));
        document.extend(element_of(0, &[(1, 3), (2, 4)]));
        let size = document.len() as u32;
        document[4..8].copy_from_slice(&size.to_le_bytes());
        document
    }

    fn pool_of(strings: &[&str], utf8_pool: bool) -> Vec<u8> {
        let mut data = Vec::new();
        let mut offsets = Vec::new();
        for string in strings {
            offsets.push(data.len() as u32);
            if utf8_pool {
                data.push(string.encode_utf16().count() as u8);
                data.push(string.len() as u8);
                data.extend_from_slice(string.as_bytes());
                data.push(0);
            } else {
                let units: Vec<u16> = string.encode_utf16().collect();
                data.extend_from_slice(&(units.len() as u16).to_le_bytes());
                for unit in units {
                    data.extend_from_slice(&unit.to_le_bytes());
                }
                data.extend_from_slice(&0u16.to_le_bytes());
            }
        }
        let aligned = data.len().div_ceil(4) * 4;
        data.resize(aligned, 0);
        let start = 28 + offsets.len() * 4;
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&0x0001u16.to_le_bytes());
        chunk.extend_from_slice(&28u16.to_le_bytes());
        chunk.extend_from_slice(&((start + data.len()) as u32).to_le_bytes());
        chunk.extend_from_slice(&(offsets.len() as u32).to_le_bytes());
        chunk.extend_from_slice(&0u32.to_le_bytes());
        chunk.extend_from_slice(&(if utf8_pool { 0x0100u32 } else { 0 }).to_le_bytes());
        chunk.extend_from_slice(&(start as u32).to_le_bytes());
        chunk.extend_from_slice(&0u32.to_le_bytes());
        for offset in offsets {
            chunk.extend_from_slice(&offset.to_le_bytes());
        }
        chunk.extend(data);
        chunk
    }

    fn element_of(name: u32, attributes: &[(u32, u32)]) -> Vec<u8> {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&0x0102u16.to_le_bytes());
        chunk.extend_from_slice(&16u16.to_le_bytes());
        chunk.extend_from_slice(&((16 + 20 + attributes.len() * 20) as u32).to_le_bytes());
        chunk.extend_from_slice(&1u32.to_le_bytes());
        chunk.extend_from_slice(&u32::MAX.to_le_bytes());
        chunk.extend_from_slice(&u32::MAX.to_le_bytes());
        chunk.extend_from_slice(&name.to_le_bytes());
        chunk.extend_from_slice(&20u16.to_le_bytes());
        chunk.extend_from_slice(&20u16.to_le_bytes());
        chunk.extend_from_slice(&(attributes.len() as u16).to_le_bytes());
        chunk.extend_from_slice(&[0u8; 6]);
        for &(key, value) in attributes {
            chunk.extend_from_slice(&u32::MAX.to_le_bytes());
            chunk.extend_from_slice(&key.to_le_bytes());
            chunk.extend_from_slice(&value.to_le_bytes());
            chunk.extend_from_slice(&8u16.to_le_bytes());
            chunk.push(0);
            chunk.push(0x03);
            chunk.extend_from_slice(&value.to_le_bytes());
        }
        chunk
    }
}
