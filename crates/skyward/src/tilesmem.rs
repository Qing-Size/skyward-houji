//! 控制中心摆哪些磁贴、按什么顺序摆，留着下一次开还是这个样子。
//!
//! 和文件管理器的排序一样放在 `/metadata/skyward/` 下：那是唯一一块跨启动
//! 留得住的、又不用碰用户数据的地方。

use anyhow::{Context, Result};
use skyward_ui::app::{Tile, from_ids, to_text};
use std::path::Path;

const FILE: &str = "/metadata/skyward/tiles";

/// 留着的摆法，如果留过；一枚都没留过时是 `None`，用默认那十二个。
pub fn read() -> Option<Vec<Tile>> {
    with_metadata(read_unmounted)
}

/// 读的时候把 /metadata 临时挂上：页面是在 recovery 挂上它之前建好的，
/// 那时候读文件就是「文件不存在」（2026-11-15 排序与 WLAN 每次都丢）。
fn with_metadata<T>(work: impl FnOnce() -> T) -> T {
    let at = Path::new("/metadata");
    let mounted = skyward_block::mount::is_mount_point(at).unwrap_or(false);
    if !mounted && skyward_block::mount::mount_ro(&Path::new(skyward_block::BY_NAME).join("metadata"), at).is_err() {
        return work();
    }
    let done = work();
    if !mounted {
        let _ = skyward_block::mount::umount(at);
    }
    done
}

fn read_unmounted() -> Option<Vec<Tile>> {
    let text = std::fs::read_to_string(FILE).ok()?;
    let tiles = from_ids(text.lines());
    // 空文件当作没留过：那多半是写到一半断电，不是用户真的一枚都不要。
    (!tiles.is_empty()).then_some(tiles)
}

/// 把摆法留着，给下一次启动用。
pub fn write(tiles: &[Tile]) -> Result<()> {
    let body = to_text(tiles);
    let at = Path::new(FILE);
    // /metadata 挂上来是只读的：写文件的那一下临时要成可写，和留着的那份
    // WLAN 一个做法。
    let _ = skyward_block::mount::remount_writable(Path::new("/metadata"), true);
    let written = std::fs::write(at, body).with_context(|| format!("write {}", at.display()));
    let _ = skyward_block::mount::remount_writable(Path::new("/metadata"), false);
    written
}
