//! 系统应用的停用与启用。
//!
//! 包管理器把「停用了哪些包」记在系统 data 分区的
//! `users/0/package-restrictions.xml` 里，那是 ABX 二进制，没有 schema 硬写
//! 会把系统的包管理弄坏（2026-11-23 之前的结论）。所以这里换一条路：把请求
//! 记成队列，装一个系统启动后执行的脚本，由系统自己的 `pm` 去改，结果写回
//! 状态文件，recovery 下次进来只读它。
//!
//! 队列与状态都放在 `/data/adb/skyward/`：那是 KernelSU/Magisk 的地盘，
//! recovery 能写，系统启动后的脚本也能读。脚本放两处，谁在就放谁那儿：
//! KernelSU metamodule 的 `boot-completed.d`（启动完成后执行）与 Magisk 的
//! `service.d`（早期执行，脚本自己等 `sys.boot_completed`）。

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const FOLDER: &str = "/data/adb/skyward";
const QUEUE: &str = "/data/adb/skyward/pm-queue";
const STATE: &str = "/data/adb/skyward/pm-state";

/// 启动脚本放在哪，**三处都不是模块**。
///
/// 以前装成模块（`/data/adb/modules/skyward-pm/`），于是它出现在 KernelSU 的
/// 模块列表里 —— 用户 2026-11-24 明确不要这个入口：这只是 Skyward 的一个后台
/// 动作，不是他装的东西。下面三个目录都是 root 管理器直接执行的脚本位，
/// 不出现在任何列表里：
///
/// 1. `/data/adb/service.d/` —— Magisk 与 KernelSU 兼容的那一层，late_start 跑。
/// 2. `/data/adb/post-fs-data.d/` —— 更早，脚本自己等系统起来。
/// 3. `/data/adb/boot-completed.d/` —— 这台机器的 KernelSU 有，顺手也放一份。
///
/// 三处都会跑一次，`pm.lock` 让先到的那个动手。
const PLACES: [&str; 3] = [
    "/data/adb/service.d/50-skyward-pm.sh",
    "/data/adb/post-fs-data.d/50-skyward-pm.sh",
    "/data/adb/boot-completed.d/50-skyward-pm.sh",
];

/// 把 2026-11-24 早先那版装成模块的目录收回来：只删自己写的那一份
/// （`module.prop` 里有 `id=skyward-pm`），别的模块一个都不碰。
fn drop_old_module() {
    let dir = Path::new("/data/adb/modules/skyward-pm");
    let prop = dir.join("module.prop");
    let ours = fs::read_to_string(&prop).map(|text| text.contains("id=skyward-pm")).unwrap_or(false);
    if ours {
        match fs::remove_dir_all(dir) {
            Ok(()) => {
                let _ = skyward_sys::kmsg("the app state module is gone; the script runs from service.d now");
            }
            Err(e) => {
                let _ = skyward_sys::kmsg(&format!("the app state module is not removed: {e}"));
            }
        }
    }
}

/// 启动后执行的脚本。只认队列里 `disable`/`enable` 两种开头的行，同一个包
/// 只留最后一条结果，跑完把队列清空。
///
/// **整段放到后台**：等 `sys.boot_completed` 要几十秒，root 管理器的
/// `service.d` 是串行执行的，前台 sleep 会把开机拖慢。后台里等，脚本立刻返回。
const SCRIPT: &str = r#"#!/system/bin/sh
# Skyward：系统应用的停用/启用队列。Skyward 在 recovery 里写队列，这里由
# 系统启动后执行 —— 包管理器的状态只有系统自己能改。
Q=/data/adb/skyward/pm-queue
S=/data/adb/skyward/pm-state
L=/data/adb/skyward/pm.log
D=/data/adb/skyward/pm.lock
[ -s "$Q" ] || exit 0
# 三个位置（模块 service.sh / service.d / boot-completed.d）会各跑一次，
# 只让先到的那一个动手，免得同一个包被连续几次 pm 折腾。
mkdir "$D" 2>/dev/null || exit 0
(
  # root 管理器的脚本环境里 PATH 常常没有 /system/bin，`pm` 于是 not found，
  # 退出码非 0 → 队列全被判失败（2026-11-24 真机：手工跑同一条命令 rc=0，
  # 脚本里却全是 -> 1）。这里把 PATH 补上，并直接拿绝对路径。
  export PATH=/system/bin:/system/xbin:/sbin:$PATH
  PM=/system/bin/pm
  [ -x "$PM" ] || PM=pm
  i=0
  while [ "$(getprop sys.boot_completed)" != "1" ]; do
    i=$((i+1))
    if [ $i -gt 600 ]; then
      echo "$(date) gave up waiting for boot completed" >> "$L" 2>/dev/null
      rmdir "$D" 2>/dev/null
      exit 0
    fi
    sleep 1
  done
  # `pm` 是 app_process 起的，**它会从 stdin 读东西**。循环用 `read ... done < "$Q"`
  # 时，pm 继承到的 stdin 就是队列文件，于是 binder 事务所回
  # `cmd: Failure calling service package: Failed transaction (2147483646)` ——
  # 这就是「停用/启用完全没作用」的最终原因（2026-11-24 真机逐条对比查出来的：
  # 同一条命令加 `< /dev/null` 就 rc=0，不加就 rc=2）。所以队列走 fd 3 读，
  # pm 的 stdin 一律指 /dev/null。
  # 光等 sys.boot_completed 不够：MIUI 把它设得比包服务就绪早，这时候 pm 会回
  # `cmd: Failure calling service package: Failed transaction (2147483646)`，队列
  # 全被判失败（2026-11-24 真机日志里就是这个）。再等到 `pm list packages` 真的
  # 能应答为止，最多四分钟。
  i=0
  until "$PM" list packages < /dev/null >/dev/null 2>&1; do
    i=$((i+1))
    if [ $i -gt 120 ]; then
      echo "$(date) package service never answered" >> "$L" 2>/dev/null
      rmdir "$D" 2>/dev/null
      exit 0
    fi
    sleep 2
  done
  sleep 3
  T="$S.new"
  if [ -f "$S" ]; then cat "$S" > "$T"; else : > "$T"; fi
  while IFS=' ' read -r op pkg rest <&3; do
    [ -n "$pkg" ] || continue
    grep -v " $pkg\$" "$T" > "$T.1" 2>/dev/null
    [ -f "$T.1" ] && mv "$T.1" "$T"
    n=0
    ok=1
    out=""
    while [ $n -lt 12 ]; do
      if [ "$op" = "disable" ]; then
        out=$("$PM" disable-user --user 0 "$pkg" < /dev/null 2>&1) && { ok=0; break; }
      elif [ "$op" = "enable" ]; then
        out=$("$PM" enable --user 0 "$pkg" < /dev/null 2>&1) && { ok=0; break; }
      else
        out="unknown op $op"
        ok=2
        break
      fi
      n=$((n+1))
      sleep 5
    done
    case "$ok" in
      0) if [ "$op" = "disable" ]; then echo "disabled $pkg" >> "$T"; else echo "enabled $pkg" >> "$T"; fi ;;
      2) ;;
      *) echo "failed $pkg" >> "$T" ;;
    esac
    echo "$(date) $op $pkg -> $ok ${out}" >> "$L" 2>/dev/null
  done 3< "$Q"
  mv "$T" "$S"
  rm -f "$Q"
  rmdir "$D" 2>/dev/null
) &
"#;

/// 系统写回的停用状态，按包名：true 是已停用。`failed` 行不留记录（那一次
/// 没成，页面上按「还能再停一次」显示）。
pub fn states() -> BTreeMap<String, bool> {
    let mut out = BTreeMap::new();
    let text = match fs::read_to_string(STATE) {
        Ok(text) => text,
        Err(_) => return out,
    };
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let (Some(word), Some(package)) = (parts.next(), parts.next()) else { continue };
        match word {
            "disabled" => out.insert(package.to_string(), true),
            "enabled" => out.insert(package.to_string(), false),
            _ => continue,
        };
    }
    out
}

/// 把一个停用/启用请求排进队列，并把启动脚本装上（已经有了就覆盖成最新
/// 那份）。data 分区没打开时这里会失败，页面据此说一句。
pub fn queue(package: &str, disable: bool) -> Result<()> {
    if package.trim().is_empty() {
        anyhow::bail!("no package to change");
    }
    let placed = ensure_script()?;
    fs::create_dir_all(FOLDER).with_context(|| format!("make {FOLDER}"))?;
    skyward_sys::kmsg(&format!("app state script is at {}", placed.join(", ")))?;
    let mut lines: Vec<String> = match fs::read_to_string(QUEUE) {
        Ok(text) => text.lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect(),
        Err(e) if e.kind() == ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e).with_context(|| format!("read {QUEUE}")),
    };
    // 同一个包只留最后一条：连点两下不该排出两条相反的操作。
    lines.retain(|line| {
        let mut parts = line.split_whitespace();
        parts.next();
        parts.next().map(|p| p != package).unwrap_or(false)
    });
    lines.push(format!("{} {package}", if disable { "disable" } else { "enable" }));
    let text = lines.join("\n") + "\n";
    let writing = format!("{QUEUE}.new");
    fs::write(&writing, text.as_bytes()).with_context(|| format!("write {writing}"))?;
    fs::rename(&writing, QUEUE).with_context(|| format!("move {writing} onto {QUEUE}"))?;
    skyward_sys::kmsg(&format!("{package} is queued to be {} at the next start", if disable { "disabled" } else { "enabled" }))?;
    Ok(())
}

/// 装上（或刷新）启动脚本，返回装到了哪些地方。
///
/// `/data/adb` 不在就是真的走不通：没有 KernelSU / Magisk，系统启动后没有任何
/// 东西会替我们跑 `pm`（`/data` 没解密也会是这个样子）。模块目录自己建 ——
/// root 管理器下次开机就会把它当模块加载。
fn ensure_script() -> Result<Vec<String>> {
    let adb = Path::new("/data/adb");
    if !adb.is_dir() {
        anyhow::bail!("/data/adb 不在（没有 KernelSU / Magisk，或者 /data 还没打开）");
    }
    drop_old_module();
    let mut placed = Vec::new();
    for place in PLACES {
        let path = Path::new(place);
        let Some(folder) = path.parent() else { continue };
        if !folder.is_dir() {
            continue;
        }
        fs::write(path, SCRIPT.as_bytes()).with_context(|| format!("write {place}"))?;
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).with_context(|| format!("make {place} runnable"))?;
        placed.push(place.to_string());
    }
    if placed.is_empty() {
        anyhow::bail!("no KernelSU/Magisk startup folder under /data/adb to run it from");
    }
    Ok(placed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 状态文件读得回来：一行一个包，`failed` 不算。
    #[test]
    fn the_states_file_reads_as_one_flag_a_package() {
        let text = "disabled com.miui.player\nenabled com.android.chrome\nfailed com.miui.video\n";
        let mut out = BTreeMap::new();
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let (Some(word), Some(package)) = (parts.next(), parts.next()) else { continue };
            match word {
                "disabled" => {
                    out.insert(package.to_string(), true);
                }
                "enabled" => {
                    out.insert(package.to_string(), false);
                }
                _ => {}
            }
        }
        assert_eq!(out.get("com.miui.player"), Some(&true));
        assert_eq!(out.get("com.android.chrome"), Some(&false));
        assert_eq!(out.get("com.miui.video"), None, "a failed run leaves no record");
    }

    /// 脚本只碰队列里那两种操作，跑完清空队列。
    /// 三处都不是模块：模块会出现在 root 管理器的模块列表里，而用户不要那个
    /// 入口（2026-11-24）。脚本位只能是 root 管理器直接执行的那些目录。
    #[test]
    fn the_script_runs_from_places_that_are_not_modules() {
        for place in PLACES {
            assert!(place.starts_with("/data/adb/"), "{place}");
            assert!(!place.contains("/modules/"), "{place} would show up as an installed module");
        }
        assert!(PLACES.iter().any(|p| p.contains("/service.d/")), "Magisk / KernelSU 都会跑这一层");
    }

    #[test]
    fn the_script_waits_for_the_system_and_clears_the_queue() {
        assert!(SCRIPT.starts_with("#!/system/bin/sh\n"));
        assert!(SCRIPT.contains("sys.boot_completed"));
        assert!(SCRIPT.contains("pm disable-user --user 0"));
        assert!(SCRIPT.contains("pm enable --user 0"));
        // 整段在后台跑（`( ... ) &`），否则 root 管理器的 service.d 会被这段
        // sleep 拖住，开机变慢；队列清空仍然是最后一步。
        assert!(SCRIPT.trim_end().ends_with(") &"));
        assert!(SCRIPT.contains("rm -f \"$Q\""));
    }
}