//! The commands install scripts written for other recoveries run that a
//! stock recovery does not carry: 7za, which takes a partition image out of
//! the zip the script came in, and zstd, which decompresses it on its way to
//! the partition; avbctl, which turns off verified boot's checks before a
//! script writes a system that would not pass them, and resetprop, which
//! reads the phone's properties and sets them; and unzip, as the stock
//! recovery's own but pouring out what `-p` takes as it reads it. The
//! ramdisk links each to the recovery service binary, which runs the one it
//! is started as. Unlike a copy of a command a script brings along, built
//! against libraries each phone's recovery carries in a version of its
//! own, these need none of them.

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::sync::mpsc;
use std::thread;

mod avbctl;
mod resetprop;
mod sevenzip;
mod unzip;
mod zstandard;

/// How much is read and written at a time: a whole partition image may go
/// through a command.
const CHUNK: usize = 1 << 20;

/// What a command reads and writes besides the files it names.
struct Io<'a> {
    input: &'a mut dyn Read,
    output: &'a mut dyn Write,
    errors: &'a mut dyn Write,
    /// The descriptor `output` writes to, where it is one: a command may
    /// then write through it on another thread while it reads, so that
    /// decompressing a partition image and writing it go on at once.
    fd: Option<RawFd>,
}

/// A command: its arguments, without its name, to the status it ends with.
type Command = fn(&[OsString], &mut Io) -> i32;

/// The command a program started as `name` is, if it is one of them.
fn command(name: &OsStr) -> Option<Command> {
    match name.to_str()? {
        "7za" => Some(sevenzip::run),
        "avbctl" => Some(avbctl::run),
        "resetprop" => Some(resetprop::run),
        "unzip" => Some(unzip::run),
        "zstd" => Some(zstandard::run),
        _ => None,
    }
}

/// Run the command this program was started as, `name`, with `args`, and
/// give the status it ends with; none when `name` is not one of them.
pub fn run(name: &OsStr, args: &[OsString]) -> Option<i32> {
    let command = command(name)?;
    // As with the programs these stand in for, writing to a pipe whose
    // reader has gone ends the command.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };
    let (stdin, stdout, stderr) = (io::stdin(), io::stdout(), io::stderr());
    let fd = stdout.as_raw_fd();
    let (mut input, mut output, mut errors) = (stdin.lock(), stdout.lock(), stderr.lock());
    Some(command(args, &mut Io { input: &mut input, output: &mut output, errors: &mut errors, fd: Some(fd) }))
}

/// Where copying stopped.
#[derive(Debug)]
enum Failed {
    Reading(io::Error),
    Writing(io::Error),
}

/// Copy what is left of `from` to `to`, giving how much there was.
fn copy(from: &mut dyn Read, to: &mut dyn Write) -> Result<u64, Failed> {
    let mut chunk = vec![0; CHUNK];
    let mut copied = 0;
    loop {
        let read = match from.read(&mut chunk) {
            Ok(0) => return Ok(copied),
            Ok(read) => read,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(Failed::Reading(e)),
        };
        to.write_all(&chunk[..read]).map_err(Failed::Writing)?;
        copied += read as u64;
    }
}

/// Copy what is left of `from` to the file `fd` names, reading on this
/// thread while another writes what was read before, so that a partition
/// image is decompressed and written at once rather than one after the
/// other. What is written through a copy of the descriptor, which the
/// caller keeps.
fn copy_to_fd(from: &mut dyn Read, fd: RawFd) -> Result<u64, Failed> {
    let copy = unsafe { libc::dup(fd) };
    if copy < 0 {
        return Err(Failed::Writing(io::Error::last_os_error()));
    }
    copy_while_writing(from, unsafe { File::from_raw_fd(copy) })
}

/// How many pieces of each kind are in flight at once in
/// [`copy_while_writing`]: one being read, and two waiting.
const IN_FLIGHT: usize = 3;

/// Copy `from` into `to` with a thread that writes what this one has read,
/// so that neither waits for the other but for the slower of them.
fn copy_while_writing(from: &mut dyn Read, mut to: File) -> Result<u64, Failed> {
    // As many places for what is free as there are pieces, so that they all
    // go in before the writing thread starts taking them.
    let (free_tx, free_rx) = mpsc::sync_channel::<Vec<u8>>(IN_FLIGHT);
    let (data_tx, data_rx) = mpsc::sync_channel::<(Vec<u8>, usize)>(IN_FLIGHT);
    thread::scope(|scope| -> Result<u64, Failed> {
        for _ in 0..IN_FLIGHT {
            free_tx.send(vec![0; CHUNK * 4]).expect("the channel is open");
        }
        let writer = scope.spawn(move || -> Result<u64, io::Error> {
            let mut written = 0;
            while let Ok((buf, n)) = data_rx.recv() {
                to.write_all(&buf[..n])?;
                written += n as u64;
                let _ = free_tx.send(buf);
            }
            Ok(written)
        });
        let mut failed = None;
        'read: while let Ok(mut buf) = free_rx.recv() {
            let read = loop {
                match from.read(&mut buf) {
                    Ok(read) => break read,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        failed = Some(Failed::Reading(e));
                        break 'read;
                    }
                }
            };
            if read == 0 {
                break;
            }
            if data_tx.send((buf, read)).is_err() {
                break;
            }
        }
        drop(data_tx);
        drop(free_rx);
        let written = writer.join().expect("the writing thread does not panic");
        match failed {
            Some(e) => Err(e),
            None => written.map_err(Failed::Writing),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn every_command_the_ramdisk_links_is_one_this_binary_runs() {
        for link in skyward_config::ramdisk::COMMANDS {
            let name = Path::new(link).file_name().expect("a link has a name");
            assert!(command(name).is_some(), "{link}");
        }
        assert!(command(OsStr::new("recovery")).is_none());
        assert!(command(OsStr::new("zipinfo")).is_none(), "the stock recovery's own answers to that name");
    }

    #[test]
    fn what_goes_to_a_descriptor_is_read_and_written_through_it() {
        use std::io::Cursor;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("super");
        let file = File::create(&path).unwrap();
        let data: Vec<u8> = (0..CHUNK + 7).map(|i| i as u8).collect();
        assert_eq!(copy_to_fd(&mut Cursor::new(&data), file.as_raw_fd()).unwrap(), data.len() as u64);
        drop(file);
        assert_eq!(std::fs::read(&path).unwrap(), data);
    }
}
