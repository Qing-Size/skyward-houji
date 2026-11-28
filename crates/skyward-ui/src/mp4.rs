//! MP4（ISO/IEC 14496-12）容器：把一帧一张 JPEG 装成 `mp4v` 轨。
//!
//! 结构是固定的：`ftyp` / `mdat`（帧数据）/ `moov`（索引）。帧一帧一帧往
//! `mdat` 里追加，收尾时再把 `moov` 写出来 —— 索引在最后，所以录制中途断电
//! 那份文件没有 `moov`，播放器打不开；这是 MP4 这种容器的固有代价。
//!
//! 一帧是一张完整的 JPEG，所以 `stsd` 里写 `mp4v`（MPEG-4 Part 2 视觉），
//! 采样条目带上 `esds` 说明基本流是 JPEG 模式。

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::PathBuf;

/// 一帧在 `mdat` 里的位置与长度。
struct Sample {
    offset: u32,
    len: u32,
}

pub struct Mp4 {
    out: BufWriter<File>,
    path: PathBuf,
    pos: u64,
    mdat_at: u64,
    /// 第一帧数据的偏移。
    data_at: u64,
    width: u32,
    height: u32,
    fps: u32,
    samples: Vec<Sample>,
    bytes: u64,
    biggest: u32,
    full: bool,
}

impl Mp4 {
    pub fn new(file: File, path: PathBuf, width: u32, height: u32, fps: u32) -> Result<Mp4> {
        let mut mp4 = Mp4 {
            out: BufWriter::with_capacity(1 << 20, file),
            path,
            pos: 0,
            mdat_at: 0,
            data_at: 0,
            width,
            height,
            fps: fps.max(1),
            samples: Vec::new(),
            bytes: 0,
            biggest: 0,
            full: false,
        };
        // ftyp：长度 24 = 8（头）+ 4（主品牌）+ 4（次版本）+ 8（兼容品牌）。
        mp4.put(&24u32.to_be_bytes())?;
        mp4.put(b"ftyp")?;
        mp4.put(b"mp42")?;
        mp4.put(&0u32.to_be_bytes())?;
        mp4.put(b"mp42")?;
        mp4.put(b"isom")?;
        // mdat：长度收尾回填。
        mp4.mdat_at = mp4.pos;
        mp4.put(&0u32.to_be_bytes())?;
        mp4.put(b"mdat")?;
        mp4.data_at = mp4.pos;
        Ok(mp4)
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn full(&self) -> bool {
        self.full
    }

    pub fn frames(&self) -> u64 {
        self.samples.len() as u64
    }

    fn put(&mut self, bytes: &[u8]) -> Result<()> {
        self.out.write_all(bytes).context("write the MP4")?;
        self.pos += bytes.len() as u64;
        Ok(())
    }

    /// 追加一帧。写满了就记下来，后面的帧不再收。
    pub fn add(&mut self, jpeg: &[u8]) -> Result<()> {
        if self.full {
            return Ok(());
        }
        // 32 位偏移：留 64 KiB 余量给 moov。
        if self.pos + jpeg.len() as u64 + 8 > u32::MAX as u64 - 65_536 {
            self.full = true;
            return Ok(());
        }
        let offset = (self.pos - self.data_at) as u32;
        self.put(jpeg)?;
        self.samples.push(Sample { offset, len: jpeg.len() as u32 });
        self.bytes += jpeg.len() as u64;
        self.biggest = self.biggest.max(jpeg.len() as u32);
        Ok(())
    }

    /// 写出 `moov`，回填 `mdat` 长度，交出文件总字节。
    pub fn finish(&mut self) -> Result<u64> {
        let end = self.pos;
        // mdat_at 就是长度字段的位置：长度包含它自己这 8 字节的头。
        self.patch32(self.mdat_at, (end - self.mdat_at) as u32)?;
        let moov = self.moov();
        self.put(&moov)?;
        self.out.flush().context("flush the MP4")?;
        Ok(self.pos)
    }

    /// 回填一个 32 位长度。`BufWriter` 上 seek 不会先把缓冲写出去，所以这里
    /// 先 flush，改完再 seek 回末尾 —— 不然补的字会落在缓冲里的旧位置。
    fn patch32(&mut self, at: u64, value: u32) -> Result<()> {
        self.out.flush().context("flush before patching the MP4")?;
        self.out.seek(SeekFrom::Start(at)).context("seek in the MP4")?;
        self.out.write_all(&value.to_be_bytes()).context("patch the MP4")?;
        self.out.flush().context("flush after patching the MP4")?;
        self.out.seek(SeekFrom::Start(self.pos)).context("seek back in the MP4")?;
        Ok(())
    }

    /// `moov` 整棵：mvhd / trak(tkhd/mdia(mdhd/hdlr/minf(vmhd/dinf/stbl)))。
    fn moov(&self) -> Vec<u8> {
        let n = self.samples.len() as u32;
        let timescale = self.fps.max(1);
        let duration = n;

        let mut mvhd = Vec::new();
        mvhd.extend_from_slice(&0u32.to_be_bytes());
        mvhd.extend_from_slice(&0u32.to_be_bytes());
        mvhd.extend_from_slice(&0u32.to_be_bytes());
        mvhd.extend_from_slice(&timescale.to_be_bytes());
        mvhd.extend_from_slice(&duration.to_be_bytes());
        mvhd.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        mvhd.extend_from_slice(&0x0100u16.to_be_bytes());
        mvhd.extend_from_slice(&[0u8; 2]);
        mvhd.extend_from_slice(&[0u8; 8]);
        for v in [0x0001_0000u32, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000] {
            mvhd.extend_from_slice(&v.to_be_bytes());
        }
        mvhd.extend_from_slice(&[0u8; 24]);
        mvhd.extend_from_slice(&2u32.to_be_bytes());

        let mut tkhd = Vec::new();
        tkhd.extend_from_slice(&0x0000_0007u32.to_be_bytes());
        tkhd.extend_from_slice(&0u32.to_be_bytes());
        tkhd.extend_from_slice(&0u32.to_be_bytes());
        tkhd.extend_from_slice(&1u32.to_be_bytes());
        tkhd.extend_from_slice(&0u32.to_be_bytes());
        tkhd.extend_from_slice(&duration.to_be_bytes());
        tkhd.extend_from_slice(&[0u8; 8]);
        tkhd.extend_from_slice(&0u16.to_be_bytes());
        tkhd.extend_from_slice(&0u16.to_be_bytes());
        tkhd.extend_from_slice(&0u16.to_be_bytes());
        tkhd.extend_from_slice(&[0u8; 2]);
        for v in [0x0001_0000u32, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000] {
            tkhd.extend_from_slice(&v.to_be_bytes());
        }
        tkhd.extend_from_slice(&(self.width << 16).to_be_bytes());
        tkhd.extend_from_slice(&(self.height << 16).to_be_bytes());

        let mut mdhd = Vec::new();
        mdhd.extend_from_slice(&0u32.to_be_bytes());
        mdhd.extend_from_slice(&0u32.to_be_bytes());
        mdhd.extend_from_slice(&0u32.to_be_bytes());
        mdhd.extend_from_slice(&timescale.to_be_bytes());
        mdhd.extend_from_slice(&duration.to_be_bytes());
        mdhd.extend_from_slice(&0x55c4u16.to_be_bytes());
        mdhd.extend_from_slice(&0u16.to_be_bytes());

        let mut hdlr = Vec::new();
        hdlr.extend_from_slice(&0u32.to_be_bytes());
        hdlr.extend_from_slice(&0u32.to_be_bytes());
        hdlr.extend_from_slice(b"vide");
        hdlr.extend_from_slice(&[0u8; 12]);
        hdlr.extend_from_slice(b"VideoHandler\0");

        let mut vmhd = Vec::new();
        vmhd.extend_from_slice(&0x0000_0001u32.to_be_bytes());
        vmhd.extend_from_slice(&[0u8; 8]);

        let mut dref = Vec::new();
        dref.extend_from_slice(&0u32.to_be_bytes());
        dref.extend_from_slice(&1u32.to_be_bytes());
        dref.extend_from_slice(&12u32.to_be_bytes());
        dref.extend_from_slice(b"url ");
        dref.extend_from_slice(&0x0000_0001u32.to_be_bytes());
        let mut dinf = Vec::new();
        dinf.extend_from_slice(&((dref.len() + 8) as u32).to_be_bytes());
        dinf.extend_from_slice(b"dref");
        dinf.extend_from_slice(&dref);

        // esds：ObjectTypeIndication 0x6C（JPEG），streamType 视觉。
        // 每个 Descriptor 的长度都用 4 字节扩展写法（80 80 80 len），
        // 免得长度超过 127 时换写法。
        let mut dec = Vec::new();
        dec.push(0x6cu8); // objectTypeIndication: JPEG
        dec.push(0x11u8); // streamType 4（视觉）<< 2 | upStream 1
        let size = self.biggest.max(1).to_be_bytes();
        dec.extend_from_slice(&[size[1], size[2], size[3]]); // bufferSizeDB
        dec.extend_from_slice(&0u32.to_be_bytes()); // maxBitrate
        dec.extend_from_slice(&0u32.to_be_bytes()); // avgBitrate
        let mut dcd = Vec::new();
        dcd.push(0x04u8);
        dcd.extend_from_slice(&[0x80, 0x80, 0x80, dec.len() as u8]);
        dcd.extend_from_slice(&dec);
        // SLConfigDescriptor：预定义 2（MP4 的默认时钟）。
        let sl = [0x06u8, 0x80, 0x80, 0x80, 0x01, 0x02];
        // ES_Descriptor 的 body：ES_ID(2) + flags(1) + 两个子描述符。
        let es_len = 2 + 1 + dcd.len() + sl.len();
        let mut esds = Vec::new();
        esds.extend_from_slice(&0u32.to_be_bytes()); // version + flags
        esds.push(0x03u8);
        esds.extend_from_slice(&[0x80, 0x80, 0x80, es_len as u8]);
        esds.extend_from_slice(&[0x00, 0x02]); // ES_ID
        esds.push(0x00); // flags
        esds.extend_from_slice(&dcd);
        esds.extend_from_slice(&sl);

        let mut entry = Vec::new();
        entry.extend_from_slice(&[0u8; 6]);
        entry.extend_from_slice(&1u16.to_be_bytes());
        entry.extend_from_slice(&[0u8; 16]);
        entry.extend_from_slice(&self.width.to_be_bytes());
        entry.extend_from_slice(&self.height.to_be_bytes());
        entry.extend_from_slice(&0x0048_0000u32.to_be_bytes());
        entry.extend_from_slice(&0x0048_0000u32.to_be_bytes());
        entry.extend_from_slice(&0u32.to_be_bytes());
        entry.extend_from_slice(&1u16.to_be_bytes());
        entry.extend_from_slice(&[0u8; 32]);
        entry.extend_from_slice(&24u16.to_be_bytes());
        entry.extend_from_slice(&0xffffu16.to_be_bytes());
        let mut stsd = Vec::new();
        stsd.extend_from_slice(&0u32.to_be_bytes());
        stsd.extend_from_slice(&1u32.to_be_bytes());
        stsd.extend_from_slice(&((entry.len() + 8) as u32).to_be_bytes());
        stsd.extend_from_slice(b"jpeg");
        stsd.extend_from_slice(&entry);

        let mut stts = Vec::new();
        stts.extend_from_slice(&0u32.to_be_bytes());
        stts.extend_from_slice(&(if n > 0 { 1u32 } else { 0 }).to_be_bytes());
        if n > 0 {
            stts.extend_from_slice(&n.to_be_bytes());
            stts.extend_from_slice(&1u32.to_be_bytes());
        }

        let mut stsc = Vec::new();
        stsc.extend_from_slice(&0u32.to_be_bytes());
        stsc.extend_from_slice(&(if n > 0 { 1u32 } else { 0 }).to_be_bytes());
        if n > 0 {
            stsc.extend_from_slice(&1u32.to_be_bytes());
            stsc.extend_from_slice(&1u32.to_be_bytes());
            stsc.extend_from_slice(&1u32.to_be_bytes());
        }

        let mut stsz = Vec::new();
        stsz.extend_from_slice(&0u32.to_be_bytes());
        stsz.extend_from_slice(&0u32.to_be_bytes());
        stsz.extend_from_slice(&n.to_be_bytes());
        for s in &self.samples {
            stsz.extend_from_slice(&s.len.to_be_bytes());
        }

        let mut stco = Vec::new();
        stco.extend_from_slice(&0u32.to_be_bytes());
        stco.extend_from_slice(&n.to_be_bytes());
        for s in &self.samples {
            stco.extend_from_slice(&s.offset.to_be_bytes());
        }

        let mut stbl = Vec::new();
        for (kind, body) in [
            (b"stsd", &stsd),
            (b"stts", &stts),
            (b"stsc", &stsc),
            (b"stsz", &stsz),
            (b"stco", &stco),
        ] {
            stbl.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
            stbl.extend_from_slice(kind);
            stbl.extend_from_slice(body);
        }

        let mut minf = Vec::new();
        for (kind, body) in [(b"vmhd", &vmhd), (b"dinf", &dinf), (b"stbl", &stbl)] {
            minf.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
            minf.extend_from_slice(kind);
            minf.extend_from_slice(body);
        }

        let mut mdia = Vec::new();
        for (kind, body) in [(b"mdhd", &mdhd), (b"hdlr", &hdlr), (b"minf", &minf)] {
            mdia.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
            mdia.extend_from_slice(kind);
            mdia.extend_from_slice(body);
        }

        let mut trak = Vec::new();
        for (kind, body) in [(b"tkhd", &tkhd), (b"mdia", &mdia)] {
            trak.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
            trak.extend_from_slice(kind);
            trak.extend_from_slice(body);
        }

        let mut moov = Vec::new();
        for (kind, body) in [(b"mvhd", &mvhd), (b"trak", &trak)] {
            moov.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
            moov.extend_from_slice(kind);
            moov.extend_from_slice(body);
        }
        let mut whole = Vec::new();
        whole.extend_from_slice(&((moov.len() + 8) as u32).to_be_bytes());
        whole.extend_from_slice(b"moov");
        whole.extend_from_slice(&moov);
        whole
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    /// 写两帧，把文件读回来按 box 走一遍：ftyp / mdat / moov 都在，长度自洽，
    /// stco 指到帧数据、stsz 说的是帧长度。
    #[test]
    fn a_file_written_holds_its_boxes_and_points_at_its_frames() {
        let dir = std::env::temp_dir().join(format!("mp4test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.mp4");
        let file = File::create(&path).unwrap();
        let mut mp4 = Mp4::new(file, path.clone(), 8, 6, 30).unwrap();
        let (a, b) = (vec![0xAAu8; 100], vec![0xBBu8; 51]);
        mp4.add(&a).unwrap();
        mp4.add(&b).unwrap();
        mp4.finish().unwrap();
        let mut bytes = Vec::new();
        File::open(&path).unwrap().read_to_end(&mut bytes).unwrap();
        // box 顺序与长度。
        let mut at = 0usize;
        let mut kinds = Vec::new();
        while at + 8 <= bytes.len() {
            let size = u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize;
            let kind = String::from_utf8_lossy(&bytes[at + 4..at + 8]).into_owned();
            assert!(size >= 8 && at + size <= bytes.len(), "{kind} size {size} at {at}");
            kinds.push((kind, size, at));
            at += size;
        }
        assert_eq!(at, bytes.len(), "boxes cover the file");
        let names: Vec<&str> = kinds.iter().map(|(k, _, _)| k.as_str()).collect();
        assert_eq!(names, ["ftyp", "mdat", "moov"]);
        // moov 里找 stco，第一条偏移应指向 mdat 数据区（ftyp 24 + mdat 头 8）。
        let moov = kinds[2].clone();
        let hay = &bytes[moov.2..moov.2 + moov.1];
        let pos = hay.windows(4).position(|w| w == b"stco").expect("an stco");
        let count = u32::from_be_bytes([hay[pos + 8], hay[pos + 9], hay[pos + 10], hay[pos + 11]]);
        assert_eq!(count, 2);
        let first = u32::from_be_bytes([hay[pos + 12], hay[pos + 13], hay[pos + 14], hay[pos + 15]]);
        assert_eq!(first, 0, "the first frame starts the data area");
        // 帧数据真在那儿。
        let data = 24 + 8;
        assert_eq!(&bytes[data..data + 100], &a[..]);
        assert_eq!(&bytes[data + 100..data + 151], &b[..]);
        std::fs::remove_dir_all(&dir).ok();
    }
}
