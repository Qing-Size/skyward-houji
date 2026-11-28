//! 彩色 emoji：手机里有两份字体管这件事。
//!
//! * `NotoColorEmoji.ttf`——COLR/CPAL 的矢量字体，一层一个字形、一种颜色，还带
//!   自己的变换、渐变与裁剪。ab_glyph 只会把轮廓化成灰度覆盖，画不出颜色也做不
//!   了变换，所以这里按层取轮廓、按层上色，先画成一张小图，再由文字绘制贴上去。
//! * `NotoColorEmojiFlags.ttf`——CBDT 位图字体，国旗在这里：两个区域指示符靠
//!   GSUB 的合字并成一面旗，单看哪一个都只是个方框。
//!
//! 两份都不一定要在 ramdisk 里：跟着已安装的系统走的那份在 `/mnt/system` 下。
//! 读不到就什么都不做，字仍是原来那样。

use ab_glyph_rasterizer::{point, Rasterizer};
use crate::canvas::Canvas;
use crate::color::Rgba;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Where the coloured typeface is looked for, in the order tried: the copy the
/// ramdisk carries, which is always there, then the installed system as the
/// recovery mounts it, and the other partitions a phone might keep it on.
const COLOR_FONTS: [&str; 6] = [
    "/skyward/NotoColorEmoji.ttf",
    "/mnt/system/fonts/NotoColorEmoji.ttf",
    "/mnt/system_ext/fonts/NotoColorEmoji.ttf",
    "/system/fonts/NotoColorEmoji.ttf",
    "/vendor/fonts/NotoColorEmoji.ttf",
    "/mnt/mi_ext/fonts/NotoColorEmoji.ttf",
];

/// The flags are a typeface of their own: a bitmap one, whose glyphs are pairs
/// of regional indicators joined by a ligature.
const FLAG_FONTS: [&str; 5] = [
    "/skyward/NotoColorEmojiFlags.ttf",
    "/mnt/system/fonts/NotoColorEmojiFlags.ttf",
    "/mnt/system_ext/fonts/NotoColorEmojiFlags.ttf",
    "/system/fonts/NotoColorEmojiFlags.ttf",
    "/vendor/fonts/NotoColorEmojiFlags.ttf",
];

/// The files the typefaces would be read from, for the log: whether the phone
/// has them at all is the first thing to know when a name comes out with a box
/// where its emoji should be.
pub fn found() -> Vec<(String, u64)> {
    COLOR_FONTS
        .iter()
        .chain(FLAG_FONTS.iter())
        .filter_map(|path| std::fs::metadata(path).ok().map(|m| (path.to_string(), m.len())))
        .collect()
}

/// 一个 emoji 画出来的样子：一张自家尺寸的小图，加上它相对笔位与基线的位置。
pub struct Art {
    pub canvas: Canvas,
    /// 图左上角相对笔位的横向偏移，像素。
    pub dx: i32,
    /// 图左上角相对基线的纵向偏移，像素（屏幕向下为正）。
    pub dy: i32,
    /// 这个 emoji 占多宽，像素：一个 em，与系统里它占的宽度一致。
    pub advance: f32,
}

/// 字体只在第一次要画 emoji 时读。第一次读不到就过一会儿再找：页面会在手机的
/// 分区还没挂上时就开始画字。
#[derive(Default)]
pub struct Slot {
    loaded: RefCell<Loaded>,
}

#[derive(Default)]
struct Loaded {
    /// When the files were last looked for, whether or not they were found.
    at: Option<Instant>,
    emoji: Option<Rc<Emoji>>,
}

/// How long to wait before looking again for typefaces that were not there.
const RETRY: Duration = Duration::from_secs(2);

impl Slot {
    /// The typefaces, read now if they have not been read yet.
    pub fn get(&self) -> Option<Rc<Emoji>> {
        let mut loaded = self.loaded.borrow_mut();
        if let Some(emoji) = &loaded.emoji {
            return Some(emoji.clone());
        }
        // 找过了没找到：别每一帧都去读那几兆的文件。
        if loaded.at.is_some_and(|at| at.elapsed() < RETRY) {
            return None;
        }
        loaded.at = Some(Instant::now());
        loaded.emoji = Emoji::read();
        loaded.emoji.clone()
    }

    /// The picture of `ch` at `px`, where a typeface draws one.
    pub fn art(&self, ch: char, px: f32) -> Option<Rc<Art>> {
        self.get()?.art(ch, px)
    }

    /// 两个区域指示符拼成的那面旗。
    pub fn flag(&self, first: char, second: char, px: f32) -> Option<Rc<Art>> {
        self.get()?.flag(first, second, px)
    }

    /// Hold `typeface` already, for a test or a preview that has one at hand
    /// rather than at one of the places above.
    pub fn keep(&self, typeface: Rc<Emoji>) {
        let mut loaded = self.loaded.borrow_mut();
        loaded.at = Some(Instant::now());
        loaded.emoji = Some(typeface);
    }
}

/// Whether `ch` is one of the characters an emoji typeface draws: the
/// pictographic blocks. The UI typeface has no shape for them by design, so the
/// test that every text the pages write is drawable leaves them to this face.
///
/// 区域指示符（U+1F1E6–U+1F1FF）也算：它们自己不是图形，两个一对才是国旗。
pub fn depicted(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1F000..=0x1FAFF
            | 0x1F1E6..=0x1F1FF
            | 0x2190..=0x21FF
            | 0x2300..=0x23FF
            | 0x25A0..=0x27BF
            | 0x2900..=0x297F
            | 0x2B00..=0x2BFF
            | 0x3030
            | 0x303D
            | 0x3297
            | 0x3299
            | 0xFE00..=0xFE0F
            | 0x200D
            | 0x20E3
    )
}

/// 区域指示符：写国旗用的那两个字母。
pub fn regional(ch: char) -> bool {
    matches!(ch as u32, 0x1F1E6..=0x1F1FF)
}

/// The emoji typefaces the phone has, and the pictures they have given already.
pub struct Emoji {
    fonts: Vec<Font>,
}

struct Font {
    /// The file's bytes, kept for the life of the process: the parsers borrow
    /// them, and a page may ask for a picture at any time.
    data: &'static [u8],
    /// 位图字体（CBDT）：旗帜那一份是这一种，字形是 PNG。
    bitmap: bool,
    /// 每个字号每个字符（或合字字形）画过一次的小图。
    cache: RefCell<HashMap<(u32, u16), Option<Rc<Art>>>>,
}

impl Emoji {
    fn read() -> Option<Rc<Emoji>> {
        let mut fonts = Vec::new();
        if let Some(font) = COLOR_FONTS.iter().find_map(|path| Font::read(Path::new(path))) {
            fonts.push(font);
        }
        if let Some(font) = FLAG_FONTS.iter().find_map(|path| Font::read(Path::new(path))) {
            fonts.push(font);
        }
        (!fonts.is_empty()).then(|| {
            Rc::new(Emoji { fonts })
        })
    }

    /// The typeface in the file at `path`, where it draws colours at all.
    pub fn read_at(path: &Path) -> Option<Rc<Emoji>> {
        Font::read(path).map(|font| Rc::new(Emoji { fonts: vec![font] }))
    }

    /// The typefaces these files hold, for a preview that has them at hand.
    pub fn read_all(paths: &[&Path]) -> Option<Rc<Emoji>> {
        let fonts: Vec<Font> = paths.iter().filter_map(|p| Font::read(p)).collect();
        (!fonts.is_empty()).then(|| Rc::new(Emoji { fonts }))
    }

    pub fn art(&self, ch: char, px: f32) -> Option<Rc<Art>> {
        self.fonts.iter().find_map(|font| font.art(ch, px))
    }

    /// 两个区域指示符拼成的那面旗：字体用 GSUB 的合字把它们并成一个字形。
    pub fn flag(&self, first: char, second: char, px: f32) -> Option<Rc<Art>> {
        for font in &self.fonts {
            let Ok(face) = ttf_parser::Face::parse(font.data, 0) else { continue };
            let (Some(a), Some(b)) = (face.glyph_index(first), face.glyph_index(second)) else { continue };
            let Some(glyph) = ligature(&face, a.0, b.0) else { continue };
            if let Some(art) = font.art_of(&face, glyph, px) {
                return Some(art);
            }
        }
        None
    }
}

/// 字号取整到像素：同一号字在整页里只画一次。
fn ppem_of(px: f32) -> u16 {
    px.round().clamp(6.0, 128.0) as u16
}

impl Font {
    fn read(path: &Path) -> Option<Font> {
        let bytes = std::fs::read(path).ok()?;
        let data: &'static [u8] = Box::leak(bytes.into_boxed_slice());
        let face = ttf_parser::Face::parse(data, 0).ok()?;
        let tables = face.tables();
        let bitmap = tables.colr.is_none();
        // 位图字形藏在四张表里的一种：CBDT（这份旗帜字体用的）、EBDT、bdat 与 sbix。
        if bitmap && tables.cbdt.is_none() && tables.ebdt.is_none() && tables.bdat.is_none() && tables.sbix.is_none() {
            return None;
        }
        Some(Font { data, bitmap, cache: RefCell::new(HashMap::new()) })
    }

    fn art(&self, ch: char, px: f32) -> Option<Rc<Art>> {
        // 这份字体里也有空格、#、* 和 0-9——键帽要用它们当底——但那些字符该由
        // 界面字体画：拿这里的图形去替换，数字就变细、变没了（2026-11-24）。
        if !depicted(ch) {
            return None;
        }
        let face = ttf_parser::Face::parse(self.data, 0).ok()?;
        let glyph = face.glyph_index(ch)?;
        self.art_of(&face, glyph.0, px)
    }

    fn art_of(&self, face: &ttf_parser::Face, glyph: u16, px: f32) -> Option<Rc<Art>> {
        let ppem = ppem_of(px);
        // 合字字形与普通字符共用一个缓存：前者从高位起，不会撞上字符码位。
        let key = (glyph as u32, ppem);
        if let Some(known) = self.cache.borrow().get(&key) {
            return known.clone();
        }
        let art = match self.bitmap {
            true => self.bitmap_art(face, glyph, ppem),
            false => self.color_art(face, glyph, ppem),
        }
        .map(Rc::new);
        self.cache.borrow_mut().insert(key, art.clone());
        art
    }

    /// 矢量那份：把每一层轮廓按调色板上色，摆到它的变换上，画进一张小图。
    fn color_art(&self, face: &ttf_parser::Face, glyph: u16, ppem: u16) -> Option<Art> {
        let colr = face.tables().colr?;
        let mut painter = Layers::default();
        // 调色板 0 是字体自己的默认配色；前景色只给那些不指调色板的层。
        let foreground = ttf_parser::RgbaColor { red: 0, green: 0, blue: 0, alpha: 255 };
        colr.paint(ttf_parser::GlyphId(glyph), 0, &mut painter, &[], foreground)?;
        if painter.shapes.is_empty() {
            return None;
        }
        let upem = face.units_per_em() as f32;
        // 按字形自己的大小画（一个 em），图比一个 em 略宽：占位也跟着图的宽度走，
        // 所以不会压到旁边的字。图高 1.2 em，基线留在下面，探到基线下头的图形
        // （桃子的底、鲸鱼的水花）才不被裁掉（0.0.78 定下来的比例）。
        let scale = ppem as f32 / upem;
        let (fw, fh) = (ppem as f32 * 1.05, ppem as f32 * 1.2);
        let (ox, oy) = (ppem as f32 * 0.02, ppem as f32);
        let mut parts: Vec<(Vec<Seg>, Rgba, Option<(f32, f32, f32, f32)>)> = Vec::new();
        for shape in &painter.shapes {
            let m = shape.transform;
            // 读坏的变换（NaN 或无穷）画出来只会把光栅化成整数时撑爆，跳过。
            if ![m.a, m.b, m.c, m.d, m.e, m.f].iter().all(|v| v.is_finite()) {
                continue;
            }
            let mut collect = Collect::default();
            if face.outline_glyph(ttf_parser::GlyphId(shape.glyph), &mut collect).is_none() {
                continue;
            }
            // 有些层的变换只说缩放与旋转、不带平移：字体自己那份 COLR 是绕字形
            // 中心转的，缺了中心就会把高光转到果实外面去。这种层补上绕自身中心
            // 的平移；带着平移的层照它自己说的摆。
            let (mut e, mut f) = (m.e, m.f);
            let turned = (m.a - 1.0).abs() > 0.01 || m.b.abs() > 0.01 || m.c.abs() > 0.01 || (m.d - 1.0).abs() > 0.01;
            if turned && e.abs() < 0.01 && f.abs() < 0.01 {
                let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
                for (x, y) in collect.segs.iter().flat_map(|s| s.points()) {
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                    y0 = y0.min(y);
                    y1 = y1.max(y);
                }
                let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                e = cx - (m.a * cx + m.c * cy);
                f = cy - (m.b * cx + m.d * cy);
            }
            let mut segs = Vec::with_capacity(collect.segs.len());
            for seg in collect.segs {
                let seg = seg.map(|x, y| {
                    let (x, y) = (m.a * x + m.c * y + e, m.b * x + m.d * y + f);
                    match shape.clip {
                        Some((a, b, c, d)) => (x.clamp(a, c), y.clamp(b, d)),
                        None => (x, y),
                    }
                });
                segs.push(seg);
            }
            parts.push((segs, shape.color, shape.clip));
        }
        if parts.is_empty() {
            return None;
        }
        let (left, bottom) = (0.0f32, 0.0f32);
        let (w, h) = (fw.ceil() as u32, fh.ceil() as u32);
        let mut canvas = Canvas::new(w, h);
        let (wf, hf) = (w as f32, h as f32);
        // 光栅化只肯在图里面画：点必须落在 0..宽 里面，外面的挤到边上（本来也
        // 要被裁掉），越界一点点就会让它的下标算溢出。
        let fix = |v: f32, hi: f32| if v.is_finite() { v.clamp(0.0, hi) } else { 0.0 };
        for (segs, color, clip) in parts {
            let clip_px = clip.map(|(x0, y0, x1, y1)| {
                let a = (ox + x0 * scale, oy - y1 * scale);
                let b = (ox + x1 * scale, oy - y0 * scale);
                (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
            });
            let mut r = Rasterizer::new(w as usize, h as usize);
            let (mut cursor, mut start) = ((0.0f32, 0.0f32), (0.0f32, 0.0f32));
            let unit = |x: f32, y: f32| (fix(ox + x * scale, wf), fix(oy - y * scale, hf));
            for seg in segs {
                match seg {
                    Seg::Move(x, y) => {
                        cursor = unit(x, y);
                        start = cursor;
                    }
                    Seg::Line(x, y) => {
                        let to = unit(x, y);
                        // 同一个点之间的一"线"会把光栅化里的斜率算成无穷，跳过。
                        if (to.0 - cursor.0).abs() > 0.01 || (to.1 - cursor.1).abs() > 0.01 {
                            r.draw_line(point(cursor.0, cursor.1), point(to.0, to.1));
                        }
                        cursor = to;
                    }
                    Seg::Quad(x1, y1, x, y) => {
                        let (c, to) = (unit(x1, y1), unit(x, y));
                        let on = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01;
                        if on(c, cursor) || on(c, to) {
                            if !on(cursor, to) {
                                r.draw_line(point(cursor.0, cursor.1), point(to.0, to.1));
                            }
                        } else {
                            r.draw_quad(point(cursor.0, cursor.1), point(c.0, c.1), point(to.0, to.1));
                        }
                        cursor = to;
                    }
                    Seg::Cubic(x1, y1, x2, y2, x, y) => {
                        let (c1, c2, to) = (unit(x1, y1), unit(x2, y2), unit(x, y));
                        let on = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01;
                        // 控制点与端点重合的三次曲线退化成一条线，按曲线算会除以零。
                        if on(c1, cursor) || on(c2, to) {
                            if !on(cursor, to) {
                                r.draw_line(point(cursor.0, cursor.1), point(to.0, to.1));
                            }
                        } else {
                            r.draw_cubic(point(cursor.0, cursor.1), point(c1.0, c1.1), point(c2.0, c2.1), point(to.0, to.1));
                        }
                        cursor = to;
                    }
                    // 轮廓要闭合才围得住里面：好几种 emoji 的层是断开的子路径。
                    Seg::Close => {
                        if (cursor.0 - start.0).abs() > 0.01 || (cursor.1 - start.1).abs() > 0.01 {
                            r.draw_line(point(cursor.0, cursor.1), point(start.0, start.1));
                        }
                        cursor = start;
                    }
                }
            }
            r.for_each_pixel_2d(|x, y, cov| {
                if let Some((x0, y0, x1, y1)) = clip_px {
                    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                    if px < x0 || px > x1 || py < y0 || py > y1 {
                        return;
                    }
                }
                canvas.blend(x as i32, y as i32, color, cov)
            });
        }
        let _ = (left, bottom);
        // 推进的宽度与图一样宽，旁边的字才不会被压住。
        Some(Art { canvas, dx: -ox.round() as i32, dy: -oy.round() as i32, advance: fw })
    }

    /// 位图那份（旗帜）：字形是 PNG，解出来按一个 em 缩放贴进小图。
    fn bitmap_art(&self, face: &ttf_parser::Face, glyph: u16, ppem: u16) -> Option<Art> {
        let image = face.glyph_raster_image(ttf_parser::GlyphId(glyph), ppem)?;
        if image.format != ttf_parser::RasterImageFormat::PNG {
            return None;
        }
        let mut decoder = png::Decoder::new(std::io::Cursor::new(image.data));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
        let mut reader = decoder.read_info().ok()?;
        let mut pixels = vec![0u8; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut pixels).ok()?;
        let (iw, ih) = (frame.width as usize, frame.height as usize);
        if iw == 0 || ih == 0 {
            return None;
        }
        let bytes = &pixels[..frame.buffer_size()];
        let pixel = |i: usize| -> Option<Rgba> {
            Some(match frame.color_type {
                png::ColorType::Rgba => Rgba { r: *bytes.get(i * 4)?, g: *bytes.get(i * 4 + 1)?, b: *bytes.get(i * 4 + 2)?, a: *bytes.get(i * 4 + 3)? },
                png::ColorType::Rgb => Rgba { r: *bytes.get(i * 3)?, g: *bytes.get(i * 3 + 1)?, b: *bytes.get(i * 3 + 2)?, a: 255 },
                png::ColorType::GrayscaleAlpha => {
                    let v = *bytes.get(i * 2)?;
                    Rgba { r: v, g: v, b: v, a: *bytes.get(i * 2 + 1)? }
                }
                png::ColorType::Grayscale => {
                    let v = *bytes.get(i)?;
                    Rgba { r: v, g: v, b: v, a: 255 }
                }
                png::ColorType::Indexed => return None,
            })
        };
        let (fw, fh) = (ppem as f32 * 1.05, ppem as f32 * 1.2);
        let (w, h) = (fw.ceil() as u32, fh.ceil() as u32);
        let mut canvas = Canvas::new(w, h);
        // 位图按 0.95 个 em 贴、横向居中；顶上留 0.11 em，与矢量那份画出来的范围
        // 对齐——旗子比别的 emoji 高出一截就是因为它原本贴得太靠上（2026-11-24）。
        let side = ppem as f32 * 0.95;
        let (ox, oy) = (ppem as f32 * 0.05, ppem as f32 * 0.11);
        for y in 0..side as u32 {
            for x in 0..side as u32 {
                let sx = ((x as f32 / side) * iw as f32) as usize;
                let sy = ((y as f32 / side) * ih as f32) as usize;
                let Some(color) = pixel(sy.min(ih - 1) * iw + sx.min(iw - 1)) else { continue };
                canvas.blend((ox as u32 + x) as i32, (oy as u32 + y) as i32, color, 1.0);
            }
        }
        Some(Art { canvas, dx: -ox.round() as i32, dy: -(ppem as f32).round() as i32, advance: fw })
    }
}

/// 两个字形能不能并成一个合字：把 GSUB 里的合字表过一遍。
fn ligature(face: &ttf_parser::Face, first: u16, second: u16) -> Option<u16> {
    let gsub = face.tables().gsub?;
    for lookup in gsub.lookups {
        for sub in lookup.subtables.into_iter::<ttf_parser::gsub::SubstitutionSubtable>() {
            let ttf_parser::gsub::SubstitutionSubtable::Ligature(lig) = sub else { continue };
            let Some(index) = lig.coverage.get(ttf_parser::GlyphId(first)) else { continue };
            let Some(set) = lig.ligature_sets.get(index) else { continue };
            for item in set {
                if item.components.len() == 1 && item.components.get(0).map(|g| g.0) == Some(second) {
                    return Some(item.glyph.0);
                }
            }
        }
    }
    None
}

/// 一段轮廓：TrueType 的二次曲线与 OpenType 的三次曲线都收。
#[derive(Clone, Copy)]
enum Seg {
    Move(f32, f32),
    Line(f32, f32),
    Quad(f32, f32, f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

impl Seg {
    /// 把每个点都过一遍 `f`：变换就这么落到轮廓上。
    fn map(self, f: impl Fn(f32, f32) -> (f32, f32)) -> Seg {
        let m = |x: f32, y: f32| f(x, y);
        match self {
            Seg::Move(x, y) => {
                let (x, y) = m(x, y);
                Seg::Move(x, y)
            }
            Seg::Line(x, y) => {
                let (x, y) = m(x, y);
                Seg::Line(x, y)
            }
            Seg::Quad(x1, y1, x, y) => {
                let (x1, y1) = m(x1, y1);
                let (x, y) = m(x, y);
                Seg::Quad(x1, y1, x, y)
            }
            Seg::Cubic(x1, y1, x2, y2, x, y) => {
                let (x1, y1) = m(x1, y1);
                let (x2, y2) = m(x2, y2);
                let (x, y) = m(x, y);
                Seg::Cubic(x1, y1, x2, y2, x, y)
            }
            Seg::Close => Seg::Close,
        }
    }

    /// 这一段落在哪些点上（控制点也算，量出来的边界略大一点而已）。
    fn points(self) -> Vec<(f32, f32)> {
        match self {
            Seg::Move(x, y) | Seg::Line(x, y) => vec![(x, y)],
            Seg::Quad(x1, y1, x, y) => vec![(x1, y1), (x, y)],
            Seg::Cubic(x1, y1, x2, y2, x, y) => vec![(x1, y1), (x2, y2), (x, y)],
            Seg::Close => Vec::new(),
        }
    }
}

#[derive(Default)]
struct Collect {
    segs: Vec<Seg>,
}

impl ttf_parser::OutlineBuilder for Collect {
    fn move_to(&mut self, x: f32, y: f32) {
        self.segs.push(Seg::Move(x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.segs.push(Seg::Line(x, y));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.segs.push(Seg::Quad(x1, y1, x, y));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.segs.push(Seg::Cubic(x1, y1, x2, y2, x, y));
    }
    fn close(&mut self) {
        self.segs.push(Seg::Close);
    }
}

/// 一层：画哪个字形、什么颜色、以及它所在的变换。
struct Shape {
    glyph: u16,
    color: Rgba,
    transform: ttf_parser::Transform,
    /// 这一层被裁到哪个框里，字体单位；`None` 是不裁。
    clip: Option<(f32, f32, f32, f32)>,
}

/// 收集 COLR 的层。`outline_glyph` 记下这一层画的是哪个字形，紧接着的 `paint`
/// 说它什么颜色；`push_transform` 与 `pop_transform` 围出这一层的变换。
#[derive(Default)]
struct Layers {
    drawn: Option<u16>,
    shapes: Vec<Shape>,
    stack: Vec<ttf_parser::Transform>,
    clips: Vec<ttf_parser::colr::ClipBox>,
    /// 进了几层 `push_layer`：那里的图层是拿来合成的（遮罩之类），不是直接
    /// 画上去的形状，按形状叠只会多出东西来。
    layers: usize,
}

impl Layers {
    /// 栈里那些裁剪框合起来：都在的时候，只有它们的交集算数。
    fn clip(&self) -> Option<(f32, f32, f32, f32)> {
        self.clips.iter().fold(None, |acc: Option<(f32, f32, f32, f32)>, c| {
            let r = (c.x_min, c.y_min, c.x_max, c.y_max);
            Some(match acc {
                None => r,
                Some(a) => (a.0.max(r.0), a.1.max(r.1), a.2.min(r.2), a.3.min(r.3)),
            })
        })
    }

    /// 栈里那些变换合起来：外层的先乘，内层的后乘。
    fn transform(&self) -> ttf_parser::Transform {
        let mut all = ttf_parser::Transform::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        for t in &self.stack {
            all = ttf_parser::Transform::combine(all, *t);
        }
        all
    }
}

impl<'a> ttf_parser::colr::Painter<'a> for Layers {
    fn outline_glyph(&mut self, glyph_id: ttf_parser::GlyphId) {
        self.drawn = Some(glyph_id.0);
    }

    fn paint(&mut self, paint: ttf_parser::colr::Paint<'a>) {
        // 一个字形画一层：这一层用完就清掉，免得下一次 paint 用错前面那个字形。
        let Some(glyph) = self.drawn.take() else { return };
        if self.layers > 0 {
            return;
        }
        let color = match &paint {
            ttf_parser::colr::Paint::Solid(c) => Some(rgba(*c)),
            // 渐变在这个字号下也只是一片色：取它第一档。
            ttf_parser::colr::Paint::LinearGradient(g) => first_stop(g.stops(0, &[])),
            ttf_parser::colr::Paint::RadialGradient(g) => first_stop(g.stops(0, &[])),
            ttf_parser::colr::Paint::SweepGradient(g) => first_stop(g.stops(0, &[])),
        };
        let Some(color) = color else { return };
        let transform = self.transform();
        let clip = self.clip();
        self.shapes.push(Shape { glyph, color, transform, clip });
    }

    fn push_clip(&mut self) {}
    fn push_clip_box(&mut self, clipbox: ttf_parser::colr::ClipBox) {
        self.clips.push(clipbox);
    }
    fn pop_clip(&mut self) {
        self.clips.pop();
    }
    fn push_layer(&mut self, _mode: ttf_parser::colr::CompositeMode) {
        self.layers += 1;
    }
    fn pop_layer(&mut self) {
        self.layers = self.layers.saturating_sub(1);
    }
    fn push_transform(&mut self, transform: ttf_parser::Transform) {
        self.stack.push(transform);
    }
    fn pop_transform(&mut self) {
        self.stack.pop();
    }
}

fn rgba(c: ttf_parser::RgbaColor) -> Rgba {
    Rgba { r: c.red, g: c.green, b: c.blue, a: c.alpha }
}

fn first_stop(stops: impl Iterator<Item = ttf_parser::colr::ColorStop>) -> Option<Rgba> {
    stops.map(|s| rgba(s.color)).next()
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLOR: &str = "/data/local/tmp/NotoColorEmoji.ttf";
    const FLAGS: &str = "/data/local/tmp/NotoColorEmojiFlags.ttf";

    #[test]
    fn a_phone_without_the_typeface_draws_no_emoji_and_breaks_nothing() {
        let slot = Slot::default();
        // 宿主上通常没有这份字体；有也只是画得出，不会出错。
        let _ = slot.art('😀', 24.0);
        let _ = slot.flag('🇨', '🇳', 24.0);
    }

    #[test]
    fn every_emoji_the_typeface_holds_draws_a_coloured_picture() {
        // 真机那两份字体可以在测试前拷到 /data/local/tmp，没有就跳过。
        if !Path::new(COLOR).exists() {
            return;
        }
        let emoji = Emoji::read_at(Path::new(COLOR)).expect("the typeface reads");
        // 数字、# 与 * 在这份字体里有字形（键帽用），但它们不该由它来画。
        for ch in ['0', '1', '5', '9', '#', '*'] {
            assert!(emoji.art(ch, 32.0).is_none(), "{ch:?} must stay with the UI face");
        }
        for ch in ['😀', '🐳', '🍑'] {
            let art = emoji.art(ch, 32.0).unwrap_or_else(|| panic!("{ch:?} draws"));
            assert!(art.canvas.w >= 16 && art.canvas.h >= 16, "{ch:?}: {}×{}", art.canvas.w, art.canvas.h);
            let coloured = art.canvas.data.as_chunks::<4>().0.iter().filter(|p| p[3] > 128).count();
            assert!(coloured > 50, "{ch:?}: only {coloured} pixels painted");
        }
        // 字体里每一个图形字符都要画得出来——一个方框都不该有。
        let face = ttf_parser::Face::parse(emoji.fonts[0].data, 0).unwrap();
        let mut missing: Vec<u32> = Vec::new();
        let mut total = 0;
        for table in face.tables().cmap.unwrap().subtables {
            if !table.is_unicode() {
                continue;
            }
            table.codepoints(|cp| {
                // 零宽连接符与变体选择符只是组合用的，自己不成图形。
                if cp == 0x200D || (0xFE00..=0xFE0F).contains(&cp) || cp == 0x20E3 {
                    return;
                }
                if !depicted(char::from_u32(cp).unwrap_or('\u{0}')) || cp < 0x2000 {
                    return;
                }
                total += 1;
                if emoji.art(char::from_u32(cp).unwrap(), 20.0).is_none() {
                    missing.push(cp);
                }
            });
        }
        assert!(total > 2000, "only {total} pictures in the typeface");
        assert!(missing.is_empty(), "{} of {total} draw nothing: {:?}", missing.len(), &missing[..missing.len().min(10)]);
    }

    #[test]
    fn a_flag_is_two_regional_indicators_joined_into_one_picture() {
        if !Path::new(FLAGS).exists() {
            return;
        }
        let emoji = Emoji::read_at(Path::new(FLAGS)).expect("the flag typeface reads");
        // 单个区域指示符在这里没有图形：它们成对才是一面旗。
        let art = emoji.flag('🇨', '🇳', 32.0).expect("China's flag draws");
        assert!(art.canvas.w >= 16 && art.canvas.h >= 16, "{}×{}", art.canvas.w, art.canvas.h);
        let coloured = art.canvas.data.as_chunks::<4>().0.iter().filter(|p| p[3] > 128).count();
        assert!(coloured > 100, "only {coloured} pixels painted");
        assert!(emoji.flag('🇨', '🏳', 32.0).is_none(), "there is no such flag");
    }
}
