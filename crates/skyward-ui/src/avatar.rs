//! The picture the phone's own system shows for its owner, read from the
//! user's photo and cut to the circle the pages draw it in.

use anyhow::{anyhow, ensure, Context, Result};
use std::path::Path;

/// A square picture, its pixels already cut to a circle: what lies outside the
/// circle is left transparent, so a page lays it over its card as it is.
#[derive(Clone, Debug)]
pub struct Avatar {
    /// The side of the square, in pixels.
    pub size: u32,
    /// `size * size` pixels of RGBA, row by row.
    pub rgba: Vec<u8>,
}

impl PartialEq for Avatar {
    /// Two of these are told apart without reading their pixels: a picture is
    /// read once, and the pages ask whether it has come, not what it shows.
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.rgba.len() == other.rgba.len() && self.rgba.as_ptr() == other.rgba.as_ptr()
    }
}

impl Avatar {
    /// Read the picture at `path` and cut it to a circle `size` pixels across.
    ///
    /// The phone keeps its owner's picture as a JPEG; the pages also take a
    /// PNG, since what a phone saves it as is not theirs to choose.
    pub fn read(path: &Path, size: u32) -> Result<Avatar> {
        ensure!(size > 0, "a picture of no width");
        let bytes = std::fs::read(path).with_context(|| format!("open {}", path.display()))?;
        let (pixels, w, h, channels) = match bytes.as_slice() {
            [0xFF, 0xD8, ..] => jpeg(&bytes, path)?,
            [0x89, b'P', b'N', b'G', ..] => png(&bytes, path)?,
            _ => anyhow::bail!("{} is neither a JPEG nor a PNG", path.display()),
        };
        let rgba = cut(&pixels, w, h, channels, size);
        Ok(Avatar { size, rgba })
    }
}

/// A PNG's pixels, normalized to eight bits a channel.
fn png(bytes: &[u8], path: &Path) -> Result<(Vec<u8>, u32, u32, usize)> {
    let _ = path;
    {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        // The phone's own picture is whatever it was saved as; the pages want
        // RGBA and nothing else.
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().with_context(|| format!("read {}", path.display()))?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).with_context(|| format!("read {}", path.display()))?;
        let (w, h) = (info.width, info.height);
        ensure!(w > 0 && h > 0, "{} is a picture of no size", path.display());
        let channels = match info.color_type {
            png::ColorType::Rgba => 4,
            png::ColorType::Rgb => 3,
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Grayscale => 1,
            other => anyhow::bail!("{} is a picture of {other:?}, which a page cannot show", path.display()),
        };
        Ok((buf, w, h, channels))
    }
}

/// A JPEG's pixels, as three channels.
fn jpeg(bytes: &[u8], path: &Path) -> Result<(Vec<u8>, u32, u32, usize)> {
    let mut decoder = zune_jpeg::JpegDecoder::new(bytes);
    // The decoder's own errors are not std ones, so they are said as they come.
    decoder.decode_headers().map_err(|e| anyhow!("read {}: {e:?}", path.display()))?;
    let (w, h) = decoder.dimensions().ok_or_else(|| anyhow!("{} is a picture of no size", path.display()))?;
    let mut pixels = decoder.decode().map_err(|e| anyhow!("read {}: {e:?}", path.display()))?;
    // A four-channel picture is cut down to three: a page wants a picture, not
    // the two bytes per pixel the phone's own saved alongside.
    if pixels.len() == w as usize * h as usize * 4 {
        let mut rgb = Vec::with_capacity(w as usize * h as usize * 3);
        for px in pixels.chunks_exact(4) {
            rgb.extend_from_slice(&px[..3]);
        }
        pixels = rgb;
    }
    ensure!(
        pixels.len() == w as usize * h as usize * 3,
        "{} is a picture of {} bytes, which is not a size",
        path.display(),
        pixels.len()
    );
    Ok((pixels, w as u32, h as u32, 3))
}

/// The picture scaled to `size` square, its corners taken off: each pixel of
/// the square is read from the one under it in the picture, and left clear when
/// it falls outside the circle.
fn cut(src: &[u8], w: u32, h: u32, channels: usize, size: u32) -> Vec<u8> {
    let mut out = vec![0u8; (size * size * 4) as usize];
    let radius = size as f32 / 2.0;
    for y in 0..size {
        for x in 0..size {
            let (dx, dy) = (x as f32 + 0.5 - radius, y as f32 + 0.5 - radius);
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            // The picture is square in practice, but one that is not is read
            // across its own width and height.
            let sx = ((x as u64 * w as u64) / size as u64) as u32;
            let sy = ((y as u64 * h as u64) / size as u64) as u32;
            let at = ((sy.min(h - 1) * w + sx.min(w - 1)) as usize) * channels;
            let into = ((y * size + x) * 4) as usize;
            let (r, g, b, a) = match channels {
                4 => (src[at], src[at + 1], src[at + 2], src[at + 3]),
                3 => (src[at], src[at + 1], src[at + 2], 255),
                2 => (src[at], src[at], src[at], src[at + 1]),
                _ => (src[at], src[at], src[at], 255),
            };
            out[into] = r;
            out[into + 1] = g;
            out[into + 2] = b;
            out[into + 3] = a;
        }
    }
    out
}

/// The name the phone gives itself, read from what its settings hold: the
/// `device_name` line of the settings file, whose bytes are a string table
/// rather than text.
///
/// The name the owner gave the phone is the one at the top of its settings,
/// which is kept as `bluetooth_name` — MIUI writes the name set there to that
/// key, since the name the phone is found by over Bluetooth is the same one.
/// Where that is absent the settings' own `device_name` is read; where nothing
/// is kept at all, the caller falls back to the model.
/// The name the phone gives itself, out of the two places the settings keep one.
///
/// 设置里的「设备名称」默认就是机型（`Xiaomi 17` 这种），没人改过时它和机型一字
/// 不差——那不是名字，是型号；用户自己起的那个在蓝牙名里，改名时不一定跟着改。
/// 所以姓名的先后是：设置里那个**改过的**名字 → 蓝牙名 → 设置里的名字
/// （2026-11-23）。
pub fn device_name(global: &Path, secure: &Path, model: Option<&str>) -> Option<String> {
    let chosen = named(global, b"device_name").filter(|name| Some(name.as_str()) != model);
    chosen
        .or_else(|| named(secure, b"bluetooth_name"))
        .or_else(|| named(secure, b"device_name"))
}

/// The string kept after `key` in the settings file at `path`.
fn named(path: &Path, key: &[u8]) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    kept(&bytes, key)
}

/// The value the settings file keeps for `key`, read as the phone wrote it.
///
/// 手机把设置值写成一段二进制：键、一个斜杠、值的类型、值本身。带 emoji 的名字
/// 是 UTF-16 大端的字节再套一层 base64，类型写 `0x0a`；其余是明文 UTF-8，类型写
/// `0x05`。所以别看内容猜 base64，看类型（真机实测 2026-11-24：😀测试 存成
/// `2D3eAG1Li9U=`，小米17 与 MyPhoneName1 都是明文）。
fn kept(bytes: &[u8], key: &[u8]) -> Option<String> {
    let at = bytes.windows(key.len()).position(|w| w == key)? + key.len();
    let mut i = at;
    // A slash may follow the key, as the phone's own format writes it.
    while i < bytes.len() && bytes[i] == b'/' {
        i += 1;
    }
    while i < bytes.len() && bytes[i] != 0 {
        i += 1;
    }
    let mut j = i;
    while j < bytes.len() && bytes[j] == 0 {
        j += 1;
    }
    // The type the value is kept as, then the length of what follows.
    let kind = *bytes.get(j)?;
    j += 1;
    while j < bytes.len() && bytes[j] == 0 {
        j += 1;
    }
    let len = *bytes.get(j)? as usize;
    j += 1;
    if len == 0 || len > 1024 {
        return None;
    }
    let raw = bytes.get(j..j + len)?;
    let name = match kind {
        // UTF-16 大端的字节，套了一层 base64。
        0x0a => utf16be(&unbase64(std::str::from_utf8(raw).ok()?)?)?,
        _ => std::str::from_utf8(raw).ok()?.to_string(),
    };
    let name = name.trim().to_string();
    if name.is_empty() || name.chars().any(char::is_control) { None } else { Some(name) }
}

/// UTF-16 大端的一串字节：奇数长度、或代理落单，都不算。
fn utf16be(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() || bytes.len() % 2 != 0 {
        return None;
    }
    let units: Vec<u16> = bytes.chunks_exact(2).map(|p| u16::from_be_bytes([p[0], p[1]])).collect();
    String::from_utf16(&units).ok()
}

/// Standard base64, or nothing where `text` is not that.
fn unbase64(text: &str) -> Option<Vec<u8>> {
    let s = text.as_bytes();
    if s.len() < 8 || s.len() % 4 != 0 {
        return None;
    }
    let pad = s.iter().rev().take_while(|&&b| b == b'=').count();
    if pad > 2 || s[..s.len() - pad].contains(&b'=') {
        return None;
    }
    let digit = |b: u8| -> Option<u32> {
        Some(match b {
            b'A'..=b'Z' => (b - b'A') as u32,
            b'a'..=b'z' => (b - b'a') as u32 + 26,
            b'0'..=b'9' => (b - b'0') as u32 + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        })
    };
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    for chunk in s.chunks(4) {
        let mut n = 0u32;
        let mut wrote = 0;
        for &b in chunk {
            n <<= 6;
            if b == b'=' {
                continue;
            }
            n |= digit(b)?;
            wrote += 1;
        }
        if wrote < 2 {
            return None;
        }
        out.push((n >> 16) as u8);
        if wrote >= 3 {
            out.push((n >> 8) as u8);
        }
        if wrote == 4 {
            out.push(n as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_with_an_emoji_is_read_out_of_the_base64_the_phone_writes() {
        // 真机实测的三段，连类型字节一起照抄：emoji 是 0x0a 加 base64，
        // 纯 ASCII 和中文都是 0x05 加明文。
        let emoji = b"\x00\x04\x00\x0ebluetooth_name/\x00\x0a\x00\x0c2D3eAG1Li9U=/\x00\x06\x00\x04root";
        assert_eq!(kept(emoji, b"bluetooth_name").as_deref(), Some("😀测试"));
        let plain = b"\x00\x04\x00\x0ebluetooth_name/\x00\x05\x00\x07Mari0us/\x00\x06\x00\x04root";
        assert_eq!(kept(plain, b"bluetooth_name").as_deref(), Some("Mari0us"));
        let chinese =
            b"\x00\x04\x00\x0ebluetooth_name/\x00\x05\x00\x08\xe5\xb0\x8f\xe7\xb1\xb317/\x00\x06\x00\x04root";
        assert_eq!(kept(chinese, b"bluetooth_name").as_deref(), Some("小米17"));
    }

    #[test]
    fn a_picture_is_cut_to_a_circle_its_corners_left_clear() {
        // Four pixels, all of them white: the circle takes the middle of them.
        let src = vec![255u8; 4 * 4 * 4];
        let out = cut(&src, 4, 4, 4, 8);
        let at = |x: u32, y: u32| out[((y * 8 + x) * 4) as usize + 3];
        assert_eq!(at(0, 0), 0, "the corner is outside the circle");
        assert_eq!(at(7, 7), 0);
        assert_eq!(at(4, 4), 255, "the middle is in it");
        assert_eq!(at(4, 0), 255, "and so is the top of it");
        assert_eq!(at(0, 4), 255, "and its left");
    }

    #[test]
    fn a_setting_that_merely_names_the_model_gives_way_to_the_bluetooth_name() {
        // 设置里那个名字就是机型：没人给手机起过名，用户起的那个在蓝牙名里。
        let mut global = vec![b'X'; 20];
        global.extend(b"\x00\x06\x00\x0bdevice_name/\x00\x07\x00\x09Xiaomi 17/");
        let mut secure = vec![b'X'; 20];
        secure.extend(b"\x00\x04\x00\x0ebluetooth_name/\x00\x05\x00\x07Mari0us/");
        let (g, s) = (std::env::temp_dir().join("skyward-avatar-model-g"), std::env::temp_dir().join("skyward-avatar-model-s"));
        std::fs::write(&g, &global).unwrap();
        std::fs::write(&s, &secure).unwrap();
        assert_eq!(device_name(&g, &s, Some("Xiaomi 17")).as_deref(), Some("Mari0us"));
        // 真的改过的名字还是它优先。
        assert_eq!(device_name(&g, &s, Some("Something else")).as_deref(), Some("Xiaomi 17"));
        let _ = std::fs::remove_file(&g);
        let _ = std::fs::remove_file(&s);
    }

    #[test]
    fn the_name_the_settings_keep_is_read_before_the_bluetooth_one() {
        // What the phone keeps: zero, index, zero, length, then the bytes.
        let mut global = vec![b'X'; 20];
        global.extend(b"\x00\x06\x00\x0bdevice_name/\x00\x07\x00\x09Xiaomi 17/");
        let mut secure = vec![b'X'; 20];
        secure.extend(b"\x00\x04\x00\x0ebluetooth_name/\x00\x05\x00\x07Mari0us/");
        let (g, s) = (std::env::temp_dir().join("skyward-avatar-global-test"), std::env::temp_dir().join("skyward-avatar-secure-test"));
        std::fs::write(&g, &global).unwrap();
        std::fs::write(&s, &secure).unwrap();
        // 设置里改的名字（global）比蓝牙名（secure）优先：改了名之后蓝牙名不一定跟着。
        assert_eq!(device_name(&g, &s, Some("Xiaomi 17")).as_deref(), Some("Mari0us"), "the setting names the model, so the Bluetooth name answers");
        assert_eq!(device_name(&g, &s, None).as_deref(), Some("Xiaomi 17"), "with no model to compare, the setting stands");
        let _ = std::fs::remove_file(&g);
        let _ = std::fs::remove_file(&s);
    }

    #[test]
    fn the_bluetooth_name_answers_where_the_settings_keep_none() {
        let mut secure = vec![b'X'; 20];
        secure.extend(b"\x00\x04\x00\x0ebluetooth_name/\x00\x05\x00\x07Mari0us/");
        let s = std::env::temp_dir().join("skyward-avatar-bt-test");
        std::fs::write(&s, &secure).unwrap();
        assert_eq!(device_name(Path::new("/nonexistent/global"), &s, Some("Xiaomi 17")).as_deref(), Some("Mari0us"));
        let _ = std::fs::remove_file(&s);
    }

    #[test]
    fn a_chinese_name_is_read_whole() {
        // The one the owner gave this phone: four characters, twelve bytes.
        let mut bytes = vec![b'X'; 20];
        bytes.extend(b"\x00\x04\x00\x0ebluetooth_name/\x00\x05\x00\x0c");
        bytes.extend("碎屏飞牛".as_bytes());
        bytes.push(b'/');
        let at = std::env::temp_dir().join("skyward-avatar-chinese-test");
        std::fs::write(&at, &bytes).unwrap();
        assert_eq!(device_name(Path::new("/nonexistent/global"), &at, None).as_deref(), Some("碎屏飞牛"));
        let _ = std::fs::remove_file(&at);
    }

    #[test]
    fn no_name_is_none() {
        let at = std::env::temp_dir().join("skyward-avatar-none-test");
        std::fs::write(&at, b"nothing here").unwrap();
        assert_eq!(device_name(&at, &at, None), None);
        let _ = std::fs::remove_file(&at);
    }
}
