//! 录屏：把页面画出来的帧缩到选好的大小、编成 JPEG，按固定帧率装进 AVI
//! （MJPEG）。
//!
//! recovery 里没有 MediaCodec，也没有 ffmpeg，能用的只有 CPU。一帧
//! 1200×2608 的 RGBA 是 12.5 MB，原画编成 JPEG 约 170–280 KB；几个线程各编
//! 各的，另有一个线程按时间轴把编好的帧写进文件。页面不是每一帧都在动，编码
//! 赶不上的时候写线程把上一帧再写一遍：播放器看到的是固定帧率，页面也不会因
//! 为等编码而卡住。
//!
//! 三个地方是 2026-11-21 那次录屏踩出来的（`Screenrecord_2026-11-21-10-45-57.avi`
//! 有 118 秒是一张不动的图）：
//!
//! * 写线程一味按时间轴补帧，追不上就一直补，越补越落后，最后到收尾才把
//!   几百兆的重复帧一口气倒出来。现在落后超过两格就把时间轴往前挪，不再补。
//! * 编码线程和写线程之间是一条只有八格的通道，写盘被安装抢了 I/O 的时候
//!   编码线程堵在 `send` 上，队列满了页面就开始丢帧，一丢就再也没恢复。
//!   现在编码结果先收下、只留最新的那一帧，谁也不堵。
//! * 一帧 1200×2608 在三个线程上只编得出 16 帧/秒，30 帧/秒的时间轴只好
//!   靠补帧填。现在默认按面板的一半录（600×1304），编码有余量，画面真在动。
//!
//! 文件是标准的 AVI：`RIFF` / `LIST hdrl`（`avih`、`strh`、`strf`）/ `LIST movi`
//! （一串 `00dc`）/ `idx1`。相机、相册、VLC、ffmpeg 都放得开。

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::canvas::Canvas;
use crate::mp4::Mp4;

/// 分辨率选项，乘在面板的宽高上：原画、四分之三、一半、八分之三。
pub const SCALES: [(u32, u32); 4] = [(1, 1), (3, 4), (1, 2), (3, 8)];

/// 画质选项，直接是 JPEG 的质量：高、中、低。
pub const QUALITIES: [u8; 3] = [88, 75, 60];

/// 帧率选项，帧每秒。
pub const RATES: [u32; 4] = [60, 30, 24, 15];

/// 容器格式：AVI（MJPEG）与 MP4（mp4v + JPEG 基本流）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Avi,
    Mp4,
}

impl Format {
    /// 文件名的后缀。
    pub fn ext(self) -> &'static str {
        match self {
            Format::Avi => "avi",
            Format::Mp4 => "mp4",
        }
    }
}

/// 默认挑的那一档：一半、中、30 帧 —— 三个编码线程各编 20 毫秒一帧，30 帧/秒
/// 有余量，每分钟约 90 MB。
pub const DEFAULT_SCALE: usize = 2;
pub const DEFAULT_QUALITY: usize = 1;
pub const DEFAULT_FPS: usize = 1;

/// 最多在内存里攒几帧等编码：再多就是攒垃圾，宁可丢。
const POOL: usize = 6;

/// AVI 的长度字段是 32 位，写到这里就停，免得到 4 GB 处把文件写坏。
const MOST_BYTES: u64 = 3_900_000_000;

/// 落后时间轴多少格就不再补帧，直接把时间轴挪到当下。
const LATE_SLOTS: u64 = 2;

/// 录一段用的大小、画质与帧率。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub quality: u8,
    pub fps: u32,
    pub format: Format,
}

impl Settings {
    /// 面板 `panel` 上的第 `scale` 档大小，第 `quality` 档画质，第 `fps` 档帧率。
    pub fn of(panel: (u32, u32), scale: usize, quality: usize, fps: usize) -> Settings {
        let (n, d) = SCALES[scale.min(SCALES.len() - 1)];
        let even = |v: u32| ((v / 2) * 2).max(2);
        Settings {
            width: even((panel.0 * n + d / 2) / d),
            height: even((panel.1 * n + d / 2) / d),
            quality: QUALITIES[quality.min(QUALITIES.len() - 1)],
            fps: RATES[fps.min(RATES.len() - 1)],
            format: Format::Avi,
        }
    }

    /// 换成另一个容器。
    pub fn with_format(mut self, format: Format) -> Settings {
        self.format = format;
        self
    }

    /// 这个大小是多少像素。
    pub fn pixels(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// 一张图大约多大，按经验值：JPEG 每像素约 `quality` 对应的一点几个比特。
    pub fn per_frame(&self) -> u64 {
        // 1200×2608 在质量 80 上实测平均 169 KB，约 0.43 bit/像素；
        // 质量高一点多一点，低一点少一点。
        let bits = match self.quality {
            0..=65 => 0.30,
            66..=80 => 0.42,
            81..=92 => 0.62,
            _ => 0.90,
        };
        (self.pixels() as f64 * bits / 8.0) as u64
    }
}

/// 一段录好的东西。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    pub path: PathBuf,
    /// 写进文件的帧数（补的帧也算）。
    pub frames: u64,
    /// 文件有多少字节。
    pub bytes: u64,
    /// 时长，按帧率算。
    pub millis: u64,
    /// 因为编码跟不上而丢掉的帧。
    pub dropped: u64,
}

/// 正在录的一段。放下它而不 [`Recorder::finish`]，写线程会把它收到一半的
/// 文件补成能放的 AVI 再退出。
pub struct Recorder {
    tx: Vec<SyncSender<Job>>,
    /// 暂停与继续交给写线程自己记：时间轴在它手上，只有它能算准暂停了多久。
    ctl: Sender<bool>,
    /// 暂停时页面连画布都不抄：一帧十二兆，抄了也是丢。
    paused: Arc<AtomicBool>,
    pool: Arc<Mutex<Vec<Vec<u8>>>>,
    /// 已经交给编码线程的帧数，也就是下一帧的序号。
    taken: Arc<AtomicU64>,
    dropped: Arc<AtomicU64>,
    writer: Option<JoinHandle<Result<Summary>>>,
    encoders: Vec<JoinHandle<()>>,
    source: (u32, u32),
    size: (u32, u32),
    path: PathBuf,
}

struct Job {
    seq: u64,
    rgba: Vec<u8>,
}

impl Recorder {
    /// 在已经打开的 `file`（写在 `path`）上开始录。`source` 是页面画布的大小，
    /// `settings` 是录出来的大小、画质与帧率。
    pub fn start(file: File, path: PathBuf, source: (u32, u32), settings: Settings) -> Result<Recorder> {
        let fps = settings.fps.max(1);
        let (w, h) = (settings.width, settings.height);
        let sink = match settings.format {
            Format::Avi => Sink::Avi(Avi::new(file, path.clone(), w, h, fps).context("write the AVI's header")?),
            Format::Mp4 => Sink::Mp4(Mp4::new(file, path.clone(), w, h, fps).context("write the MP4's header")?),
        };
        let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let count = (cores / 2).clamp(2, 4);
        let pool: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
        // 编好的帧交给写线程，按序号排好；通道短，编码慢了就让写线程补帧，
        // 不把内存堆起来。
        let (done_tx, done_rx) = mpsc::sync_channel::<(u64, Vec<u8>)>(count * 2 + 2);
        let (ctl, ctl_rx) = mpsc::channel::<bool>();
        let paused = Arc::new(AtomicBool::new(false));
        let taken = Arc::new(AtomicU64::new(0));
        let dropped = Arc::new(AtomicU64::new(0));
        let xs = spans(source.0, w);
        let ys = spans(source.1, h);
        let mut tx = Vec::with_capacity(count);
        let mut encoders = Vec::with_capacity(count);
        for at in 0..count {
            let (job_tx, job_rx) = mpsc::sync_channel::<Job>(1);
            tx.push(job_tx);
            let done_tx = done_tx.clone();
            let pool = Arc::clone(&pool);
            let (xs, ys) = (xs.clone(), ys.clone());
            let (sw, sh) = source;
            let quality = settings.quality;
            encoders.push(
                std::thread::Builder::new()
                    .name(format!("record encode {at}"))
                    .spawn(move || {
                        while let Ok(job) = job_rx.recv() {
                            let mut rgb = Vec::new();
                            shrink(&job.rgba, sw as usize, sh as usize, &xs, &ys, &mut rgb);
                            match encode(&rgb, w, h, quality) {
                                Ok(jpeg) => {
                                    // 写线程不再收新的也没关系：这一帧丢了，
                                    // 页面不等它。
                                    let _ = done_tx.send((job.seq, jpeg));
                                }
                                Err(e) => {
                                    let _ = skyward_kmsg(&format!("record: a frame was not encoded: {e:#}"));
                                }
                            }
                            // 缓冲还回池子：每帧 12 MB，不能每帧都向系统要。
                            if let Ok(mut pool) = pool.lock()
                                && pool.len() < POOL
                            {
                                pool.push(job.rgba);
                            }
                        }
                    })
                    .context("start a thread to encode the frames")?,
            );
        }
        drop(done_tx);
        let writer = std::thread::Builder::new()
            .name("record mux".into())
            .spawn(move || mux(sink, done_rx, ctl_rx, fps))
            .context("start the thread that writes the file")?;
        Ok(Recorder {
            tx,
            ctl,
            paused,
            pool,
            taken,
            dropped,
            writer: Some(writer),
            encoders,
            source,
            size: (w, h),
            path,
        })
    }

    /// 这一段写到哪个文件。
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// 录出来的画面有多宽多高。
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// 暂停或继续这一段。暂停期间页面不再抄画布，写线程也不写格子；继续之后
    /// 时间轴从停下的地方接上，文件里不会有空档，也不会有停住的一张图。
    pub fn set_paused(&self, on: bool) {
        self.paused.store(on, Ordering::Relaxed);
        let _ = self.ctl.send(on);
    }

    /// 页面画完一帧就叫一次：画布整块抄进回收来的缓冲，交给一条闲着的编码
    /// 线程。抄一块内存比逐像素去掉 alpha 快一个数量级，缩放与编码都在别的
    /// 线程上做，页面这一下只花两三毫秒。
    pub fn push(&self, canvas: &Canvas) {
        if self.paused.load(Ordering::Relaxed) {
            return;
        }
        let (w, h) = (canvas.w, canvas.h);
        if (w, h) != self.source || canvas.data.len() != (w as usize) * (h as usize) * 4 {
            return;
        }
        let mut job = self.spare(canvas.data.len());
        job.rgba.copy_from_slice(&canvas.data);
        let seq = self.taken.load(Ordering::Relaxed);
        job.seq = seq;
        // 轮流交给各条编码线程：哪条手上是空的就给哪条，都忙就丢这一帧。
        let count = self.tx.len();
        for k in 0..count {
            let at = (seq as usize + k) % count;
            match self.tx[at].try_send(job) {
                Ok(()) => {
                    self.taken.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                Err(TrySendError::Full(back)) | Err(TrySendError::Disconnected(back)) => job = back,
            }
        }
        self.dropped.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut pool) = self.pool.lock()
            && pool.len() < POOL
        {
            pool.push(job.rgba);
        }
    }

    /// 从池子里拿一个够大的缓冲，没有就新开一个。
    fn spare(&self, len: usize) -> Job {
        let mut rgba = self.pool.lock().ok().and_then(|mut pool| pool.pop()).unwrap_or_default();
        if rgba.len() != len {
            rgba = vec![0u8; len];
        }
        Job { seq: 0, rgba }
    }

    /// 收尾：不再收帧，等编码线程把剩下的编完，写线程把索引补上。
    pub fn finish(mut self) -> Result<Summary> {
        self.tx.clear();
        for e in self.encoders.drain(..) {
            let _ = e.join();
        }
        let summary = match self.writer.take() {
            Some(w) => w.join().map_err(|_| anyhow::anyhow!("the thread that writes the file panicked"))??,
            None => anyhow::bail!("the file was already written"),
        };
        Ok(Summary { dropped: self.dropped.load(Ordering::Relaxed) + summary.dropped, ..summary })
    }
}

impl Drop for Recorder {
    /// 页面没叫 `finish` 就把它丢了：把通道关掉，让写线程自己收尾。这里不等
    /// 线程结束 —— 丢下它的地方可能是页面的收尾路径，等下去只会更难收拾。
    fn drop(&mut self) {
        self.tx.clear();
    }
}

/// 一帧 RGBA 缩成 `xs`×`ys` 那样多格，每格取平均数，写成 RGB。文字缩小时
/// 取平均比丢点清楚得多：丢点会让笔画断，平均只是糊。
fn shrink(src: &[u8], sw: usize, _sh: usize, xs: &[(u32, u32)], ys: &[(u32, u32)], out: &mut Vec<u8>) {
    let (dw, dh) = (xs.len(), ys.len());
    out.clear();
    out.resize(dw * dh * 3, 0);
    if xs.iter().all(|&(_, n)| n == 1) && ys.iter().all(|&(_, n)| n == 1) {
        // 原画：只去掉 alpha，按下标写，比逐像素 extend 快。
        let mut at = 0;
        for px in src.chunks_exact(4) {
            out[at] = px[0];
            out[at + 1] = px[1];
            out[at + 2] = px[2];
            at += 3;
        }
        return;
    }
    let mut at = 0;
    for &(y0, ny) in ys {
        for &(x0, nx) in xs {
            let mut sum = [0u32; 3];
            for y in y0..y0 + ny {
                let row = ((y as usize) * sw + x0 as usize) * 4;
                for x in 0..nx as usize {
                    let p = row + x * 4;
                    sum[0] += src[p] as u32;
                    sum[1] += src[p + 1] as u32;
                    sum[2] += src[p + 2] as u32;
                }
            }
            let n = (nx * ny).max(1);
            out[at] = (sum[0] / n) as u8;
            out[at + 1] = (sum[1] / n) as u8;
            out[at + 2] = (sum[2] / n) as u8;
            at += 3;
        }
    }
}

/// 目标 `dst` 每一格取源 `src` 的哪一段：起点与格数。
fn spans(src: u32, dst: u32) -> Vec<(u32, u32)> {
    let dst = dst.max(1);
    (0..dst)
        .map(|d| {
            let a = (d as u64 * src as u64 / dst as u64) as u32;
            let b = (((d as u64 + 1) * src as u64 + dst as u64 - 1) / dst as u64) as u32;
            let b = b.max(a + 1).min(src);
            (a, b - a)
        })
        .collect()
}

/// 把 RGB 编成 JPEG。`image` 的编码器按 4:2:2 采样，文字边缘够清楚。
fn encode(rgb: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(rgb.len() / 8);
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
    encoder.encode(rgb, width, height, image::ExtendedColorType::Rgb8).context("encode a JPEG frame")?;
    Ok(out)
}

/// 两种容器共用的几个动作。
enum Sink {
    Avi(Avi),
    Mp4(Mp4),
}

impl Sink {
    fn path(&self) -> PathBuf {
        match self {
            Sink::Avi(a) => a.path(),
            Sink::Mp4(m) => m.path(),
        }
    }

    fn bytes(&self) -> u64 {
        match self {
            Sink::Avi(a) => a.bytes(),
            Sink::Mp4(m) => m.bytes(),
        }
    }

    /// 还能不能再收一帧。
    fn has_room(&self) -> bool {
        match self {
            Sink::Avi(a) => a.bytes() < MOST_BYTES,
            Sink::Mp4(m) => !m.full(),
        }
    }

    fn add(&mut self, jpeg: &[u8]) -> Result<()> {
        match self {
            Sink::Avi(a) => a.add(jpeg),
            Sink::Mp4(m) => m.add(jpeg),
        }
    }

    fn frames(&self) -> u64 {
        match self {
            Sink::Avi(a) => a.frames,
            Sink::Mp4(m) => m.frames(),
        }
    }

    fn finish(&mut self) -> Result<u64> {
        match self {
            Sink::Avi(a) => a.finish(),
            Sink::Mp4(m) => m.finish(),
        }
    }
}

/// 写线程：按时间轴把编好的帧写进容器。
///
/// 时间轴是固定的：一格一帧，编号对得上的那一帧还没来就把上一帧再写一遍，
/// 所以播放器看到的时长和录的时长一致，编码慢只是动作少，不是快进。
///
/// 追不上的时候不补：写盘被安装抢了 I/O 时，一格 200 KB 的重复帧只会让落后
/// 越来越长 —— 那天就是这么攒出 118 秒不动的一张图的。落后超过
/// [`LATE_SLOTS`] 格就把时间轴挪到当下，画面照录，文件短一点。
fn mux(mut sink: Sink, rx: Receiver<(u64, Vec<u8>)>, ctl: Receiver<bool>, fps: u32) -> Result<Summary> {
    let step = Duration::from_millis(1000) / fps;
    let step_secs = 1.0 / fps as f64;
    let mut newest: Option<Vec<u8>> = None;
    let mut last: Vec<u8> = Vec::new();
    let mut closed = false;
    let mut written = 0u64;
    let mut stale = 0u64;
    let mut skipped = 0u64;
    let mut full = false;
    let mut started: Option<Instant> = None;
    // 暂停：`paused_at` 是停下的时刻，`held` 是这一段一共停了多久。时间轴按
    // 「此刻 - 开始 - 停过的」算，所以继续之后接着写，不留空档。
    let mut paused_at: Option<Instant> = None;
    let mut held = Duration::ZERO;
    let mut paused_for = 0u64;
    loop {
        while let Ok(on) = ctl.try_recv() {
            match (on, paused_at) {
                (true, None) => paused_at = Some(Instant::now()),
                (false, Some(at)) => {
                    held += at.elapsed();
                    paused_at = None;
                    paused_for += 1;
                }
                _ => {}
            }
        }
        // 先把通道里的都收下来，只留最新的：编码快过写盘时，画面宁可跳一格，
        // 也不能把队列塞满 —— 塞满了编码线程就停在那里，之后永远是旧画面。
        loop {
            match rx.try_recv() {
                Ok((_, jpeg)) => {
                    if newest.replace(jpeg).is_some() {
                        stale += 1;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    closed = true;
                    break;
                }
            }
        }
        if started.is_none() {
            // 时间轴从第一帧开始，不从打开文件开始：页面刚开还没画的时候
            // 不该先记下几格空白。
            if newest.is_none() {
                if closed {
                    break;
                }
                match rx.recv_timeout(Duration::from_millis(40)) {
                    Ok((_, jpeg)) => newest = Some(jpeg),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => closed = true,
                }
                continue;
            }
            started = Some(Instant::now());
        }
        let started = started.expect("set above");
        let now = Instant::now();
        // 暂停着：不写格子，也不让时间轴往前走。手上那一帧留着，继续时先写它。
        if paused_at.is_some() {
            // 暂停着就收尾：不再等这一帧，否则 `finish` 会一直等这条线程回去。
            if closed {
                break;
            }
            match rx.recv_timeout(Duration::from_millis(20)) {
                Ok((_, jpeg)) => {
                    if newest.replace(jpeg).is_some() {
                        stale += 1;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => closed = true,
            }
            continue;
        }
        let played = (now - started).saturating_sub(held);
        let elapsed = played.as_secs_f64();
        // 落后太多：把时间轴挪到当下，那些格子不再补。
        let behind = (elapsed / step_secs) as u64;
        if behind > written + LATE_SLOTS {
            skipped += behind - written;
            written = behind;
        }
        if closed && newest.is_none() {
            break;
        }
        let due = started + held + step * written as u32;
        if now < due {
            let wait = due.saturating_duration_since(now).min(Duration::from_millis(20));
            match rx.recv_timeout(wait) {
                Ok((_, jpeg)) => {
                    if newest.replace(jpeg).is_some() {
                        stale += 1;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => closed = true,
            }
            continue;
        }
        // 该写这一格了：有新的就写新的，没有就把上一帧再写一遍。
        let jpeg = match newest.take() {
            Some(jpeg) => {
                last = jpeg;
                Some(&last)
            }
            None if !last.is_empty() => Some(&last),
            None => None,
        };
        if let Some(jpeg) = jpeg {
            if !full && sink.has_room() {
                sink.add(jpeg)?;
            } else if !full {
                full = true;
                let _ = skyward_kmsg("record: the file is as big as its container can be; the frames after this are left out");
            }
        }
        written += 1;
        if full {
            std::thread::sleep(step);
        }
    }
    let path = sink.path();
    let frames = sink.frames();
    let bytes = sink.finish()?;
    let millis = (written as u128 * 1000 / fps as u128) as u64;
    let _ = skyward_kmsg(&format!(
        "record: {frames} frames, {:.1} s, {:.1} MB, {:.1} MB/s in {} ({stale} newer frames replaced, {skipped} slots skipped, {paused_for} pauses)",
        millis as f64 / 1000.0,
        bytes as f64 / 1e6,
        bytes as f64 / 1e6 / (millis as f64 / 1000.0).max(0.001),
        path.display(),
    ));
    Ok(Summary { path, frames, bytes, millis, dropped: stale })
}

/// 打一条内核日志。`skyward-sys` 不在这个 crate 的依赖里（UI 要在主机上也能
/// 编），所以录屏自己写 `/dev/kmsg`，写不了就算了。
fn skyward_kmsg(line: &str) -> std::io::Result<()> {
    use std::io::Write as _;
    let mut f = std::fs::OpenOptions::new().write(true).open("/dev/kmsg")?;
    writeln!(f, "<6>skyward: {line}")
}

/// AVI 文件：先占住长度字段，写完回来补。
struct Avi {
    out: BufWriter<File>,
    path: PathBuf,
    fps: u32,
    pos: u64,
    riff_at: u64,
    hdrl_at: u64,
    strl_at: u64,
    movi_list_at: u64,
    movi_at: u64,
    avih_at: u64,
    strh_at: u64,
    frames: u64,
    biggest: u32,
    bytes: u64,
    index: Vec<(u32, u32)>,
}

impl Avi {
    fn new(file: File, path: PathBuf, w: u32, h: u32, fps: u32) -> Result<Avi> {
        let mut avi = Avi {
            out: BufWriter::with_capacity(1 << 20, file),
            path,
            fps,
            pos: 0,
            riff_at: 0,
            hdrl_at: 0,
            strl_at: 0,
            movi_list_at: 0,
            movi_at: 0,
            avih_at: 0,
            strh_at: 0,
            frames: 0,
            biggest: 0,
            bytes: 0,
            index: Vec::new(),
        };
        avi.riff_at = 0;
        avi.put(b"RIFF")?;
        avi.put(&0u32.to_le_bytes())?;
        avi.put(b"AVI ")?;

        avi.hdrl_at = avi.pos;
        avi.put(b"LIST")?;
        avi.put(&0u32.to_le_bytes())?;
        avi.put(b"hdrl")?;

        avi.avih_at = avi.pos;
        avi.put(b"avih")?;
        avi.put(&56u32.to_le_bytes())?;
        let step = (1_000_000.0 / fps as f64).round() as u32;
        avi.put(&step.to_le_bytes())?; // dwMicroSecPerFrame
        avi.put(&0u32.to_le_bytes())?; // dwMaxBytesPerSec，收尾补
        avi.put(&0u32.to_le_bytes())?; // dwPaddingGranularity
        avi.put(&0x10u32.to_le_bytes())?; // AVIF_HASINDEX
        avi.put(&0u32.to_le_bytes())?; // dwTotalFrames，收尾补
        avi.put(&0u32.to_le_bytes())?; // dwInitialFrames
        avi.put(&1u32.to_le_bytes())?; // dwStreams
        avi.put(&0u32.to_le_bytes())?; // dwSuggestedBufferSize，收尾补
        avi.put(&w.to_le_bytes())?;
        avi.put(&h.to_le_bytes())?;
        avi.put(&[0u8; 16])?; // dwReserved[4]

        avi.strl_at = avi.pos;
        avi.put(b"LIST")?;
        avi.put(&0u32.to_le_bytes())?;
        avi.put(b"strl")?;

        avi.strh_at = avi.pos;
        avi.put(b"strh")?;
        avi.put(&56u32.to_le_bytes())?;
        avi.put(b"vids")?;
        avi.put(b"MJPG")?;
        avi.put(&0u32.to_le_bytes())?; // dwFlags
        avi.put(&0u16.to_le_bytes())?; // wPriority
        avi.put(&0u16.to_le_bytes())?; // wLanguage
        avi.put(&0u32.to_le_bytes())?; // dwInitialFrames
        avi.put(&1u32.to_le_bytes())?; // dwScale
        avi.put(&fps.to_le_bytes())?; // dwRate
        avi.put(&0u32.to_le_bytes())?; // dwStart
        avi.put(&0u32.to_le_bytes())?; // dwLength，收尾补
        avi.put(&0u32.to_le_bytes())?; // dwSuggestedBufferSize，收尾补
        avi.put(&u32::MAX.to_le_bytes())?; // dwQuality
        avi.put(&0u32.to_le_bytes())?; // dwSampleSize
        avi.put(&0i16.to_le_bytes())?; // rcFrame.left
        avi.put(&0i16.to_le_bytes())?; // rcFrame.top
        avi.put(&(w as i16).to_le_bytes())?; // rcFrame.right
        avi.put(&(h as i16).to_le_bytes())?; // rcFrame.bottom

        avi.put(b"strf")?;
        avi.put(&40u32.to_le_bytes())?;
        avi.put(&40u32.to_le_bytes())?; // biSize
        avi.put(&w.to_le_bytes())?; // biWidth
        avi.put(&h.to_le_bytes())?; // biHeight
        avi.put(&1u16.to_le_bytes())?; // biPlanes
        avi.put(&24u16.to_le_bytes())?; // biBitCount
        avi.put(b"MJPG")?; // biCompression
        avi.put(&(w.saturating_mul(h).saturating_mul(3)).to_le_bytes())?; // biSizeImage
        avi.put(&0u32.to_le_bytes())?;
        avi.put(&0u32.to_le_bytes())?;
        avi.put(&0u32.to_le_bytes())?;
        avi.put(&0u32.to_le_bytes())?;

        avi.movi_list_at = avi.pos;
        avi.put(b"LIST")?;
        avi.put(&0u32.to_le_bytes())?;
        avi.movi_at = avi.pos;
        avi.put(b"movi")?;
        Ok(avi)
    }

    fn path(&self) -> PathBuf {
        self.path.clone()
    }

    fn bytes(&self) -> u64 {
        self.bytes
    }

    /// 往 `movi` 里追加一帧：一个 `00dc` 块，奇数长度补一个 0 字节。
    fn add(&mut self, jpeg: &[u8]) -> Result<()> {
        let at = self.pos;
        self.put(b"00dc")?;
        self.put(&(jpeg.len() as u32).to_le_bytes())?;
        self.put(jpeg)?;
        if jpeg.len() % 2 == 1 {
            self.put(&[0u8])?;
        }
        self.index.push(((at - self.movi_at) as u32, jpeg.len() as u32));
        self.frames += 1;
        self.bytes += jpeg.len() as u64;
        self.biggest = self.biggest.max(jpeg.len() as u32);
        Ok(())
    }

    fn put(&mut self, bytes: &[u8]) -> Result<()> {
        self.out.write_all(bytes).context("write the AVI")?;
        self.pos += bytes.len() as u64;
        Ok(())
    }

    /// 补长度与索引，交出文件有多少字节。帧数按真写进去的那些来算 ——
    /// 2026-11-21 那次的头里写着 5373 帧，`idx1` 只有 5017 条，就是这么来的。
    fn finish(&mut self) -> Result<u64> {
        let index_at = self.pos;
        let index = std::mem::take(&mut self.index);
        let frames = index.len() as u64;
        self.put(b"idx1")?;
        self.put(&((index.len() * 16) as u32).to_le_bytes())?;
        for (offset, len) in &index {
            self.put(b"00dc")?;
            self.put(&0x10u32.to_le_bytes())?; // AVIIF_KEYFRAME
            self.put(&offset.to_le_bytes())?;
            self.put(&len.to_le_bytes())?;
        }
        let end = self.pos;
        // movi 的 LIST 长度：从 'movi' 之后到 idx1 之前。
        let movi_len = (index_at - (self.movi_list_at + 8)) as u32;
        let hdrl_len = (self.movi_list_at - (self.hdrl_at + 8)) as u32;
        // strl 的 LIST 长度：从 'strl' 之后到 LIST movi 之前。
        let strl_len = (self.movi_list_at - (self.strl_at + 8)) as u32;
        self.out.flush().context("flush the AVI")?;
        self.patch32(self.riff_at + 4, (end - 8) as u32)?;
        self.patch32(self.hdrl_at + 4, hdrl_len)?;
        self.patch32(self.strl_at + 4, strl_len)?;
        self.patch32(self.movi_list_at + 4, movi_len)?;
        // avih：总帧数、每秒最大字节、建议缓冲。
        let every = (self.fps as u64 * (self.bytes.max(1) / frames.max(1)) as u64).min(u32::MAX as u64);
        self.patch32(self.avih_at + 8 + 4, every as u32)?;
        self.patch32(self.avih_at + 8 + 16, frames as u32)?;
        self.patch32(self.avih_at + 8 + 28, self.biggest)?;
        // strh：长度、建议缓冲。
        self.patch32(self.strh_at + 8 + 32, frames as u32)?;
        self.patch32(self.strh_at + 8 + 36, self.biggest)?;
        self.out.flush().context("flush the AVI")?;
        Ok(end)
    }

    fn patch32(&mut self, at: u64, value: u32) -> Result<()> {
        self.out.seek(SeekFrom::Start(at)).context("seek in the AVI")?;
        self.out.write_all(&value.to_le_bytes()).context("patch the AVI")?;
        self.out.seek(SeekFrom::Start(self.pos)).context("seek back in the AVI")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一段 AVI 的头里，帧数、宽高与索引都按写进去的来。
    #[test]
    fn a_finished_avi_names_its_frames_and_its_size() {
        let path = PathBuf::from("/tmp/skyward-record-test.avi");
        let mut avi = Avi::new(File::create(&path).unwrap(), path, 1080, 2400, 30).unwrap();
        avi.put(b"00dc").unwrap();
        avi.put(&4u32.to_le_bytes()).unwrap();
        avi.put(&[0xff, 0xd8, 0xff, 0xd9]).unwrap();
        avi.index.push((4, 4));
        avi.frames += 1;
        avi.bytes += 4;
        avi.biggest = 4;
        let bytes = avi.finish().unwrap();
        let text = std::fs::read("/tmp/skyward-record-test.avi").unwrap();
        assert_eq!(bytes as usize, text.len());
        assert_eq!(&text[..4], b"RIFF");
        assert!(text.windows(4).any(|w| w == b"idx1"));
        let frames = u32::from_le_bytes(text[0x30..0x34].try_into().unwrap());
        assert_eq!(frames, 1);
    }

    /// 每一档大小都按面板算出来，宽高都是偶数。
    #[test]
    fn every_scale_is_an_even_share_of_the_panel() {
        let panel = (1200, 2608);
        assert_eq!(Settings::of(panel, 0, 1, 1).width, 1200);
        assert_eq!(Settings::of(panel, 0, 1, 1).height, 2608);
        assert_eq!(Settings::of(panel, 1, 1, 1).width, 900);
        assert_eq!(Settings::of(panel, 1, 1, 1).height, 1956);
        assert_eq!(Settings::of(panel, 2, 1, 1).width, 600);
        assert_eq!(Settings::of(panel, 2, 1, 1).height, 1304);
        assert_eq!(Settings::of(panel, 3, 1, 1).width, 450);
        assert_eq!(Settings::of(panel, 3, 1, 1).height, 978);
    }

    /// 缩放：一整块平均，颜色算对。
    #[test]
    fn shrinking_averages_whole_blocks() {
        // 4×2 全红，缩成 2×1：每格四块，还是红。
        let src = vec![255u8, 0, 0, 255].repeat(8);
        let (xs, ys) = (spans(4, 2), spans(2, 1));
        let mut out = Vec::new();
        shrink(&src, 4, 2, &xs, &ys, &mut out);
        assert_eq!(out, vec![255, 0, 0, 255, 0, 0]);
        // 左黑右白，缩成 1×1：一半灰。
        let mut src = vec![0u8, 0, 0, 255].repeat(4);
        src.extend(vec![255u8, 255, 255, 255].repeat(4));
        let mut out = Vec::new();
        shrink(&src, 4, 2, &spans(4, 1), &spans(2, 1), &mut out);
        assert_eq!(out, vec![127, 127, 127]);
    }
}
