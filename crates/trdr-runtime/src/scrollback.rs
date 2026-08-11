//! What the agent printed, kept as the bytes it printed.
//!
//! Scrollback that survives an app restart cannot live in the WebView, so it is
//! written to a file beside the rest of the runtime state. Nothing here decodes,
//! splits, or reads the stream: the host owns the terminal but must not turn its
//! output into product state (`docs/FOUNDATION_DESIGN.md` section 11), so
//! persistence has to stay a byte copy and nothing else.
//!
//! # What the bound is, and what happens at it
//!
//! Two numbers, and they are deliberately different. [`KEEP_BYTES`] — one
//! mebibyte — is how much is replayed into a fresh terminal, which is a few
//! thousand lines and about as far back as a person scrolls. [`TRIM_AT`] — four
//! mebibytes — is how large the file is allowed to grow before it is rewritten
//! down to that tail.
//!
//! At the bound the **oldest** bytes go and the newest stay. That direction is
//! the only one that is safe: a terminal stream is a state machine, and the end
//! of it is the part that describes the screen as it is now. Dropping the newest
//! bytes instead would leave a replay that reconstructs a screen the agent has
//! already moved on from.
//!
//! Trimming can cut an escape sequence or a multi-byte character in half, and
//! that is accepted rather than avoided. The alternative is to look for sequence
//! boundaries, which means parsing the stream — the one thing this file exists
//! to not do. A terminal resynchronises on the next complete sequence, so the
//! cost is at most a wrong glyph on the first replayed line.
//!
//! Every operation here is best-effort. A scrollback that cannot be written is
//! a smaller problem than a terminal that refuses to run because of it, so a
//! failure to open, write, or trim leaves the live session alone.

use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// How much of the tail is kept, and replayed into a new terminal.
pub const KEEP_BYTES: u64 = 1024 * 1024;

/// How large the file may grow before it is rewritten down to [`KEEP_BYTES`].
///
/// Four times the kept size rather than one, so that trimming happens once per
/// three mebibytes written instead of on every append.
pub const TRIM_AT: u64 = 4 * KEEP_BYTES;

/// One terminal's scrollback file.
///
/// The path is given rather than discovered, for the reason `commands.rs` states
/// about handlers: what is decided once, at start-up, is not rediscovered later.
/// It also means a test proves the round trip without going near a person's real
/// `~/.trdr`.
#[derive(Debug, Clone)]
pub struct Scrollback
{
    file: PathBuf
}

impl Scrollback
{
    /// The scrollback kept in this file.
    pub fn at(file: impl Into<PathBuf>) -> Self
    {
        Self { file: file.into() }
    }

    /// Where it is kept.
    pub fn path(&self) -> &Path
    {
        &self.file
    }

    /// Adds bytes to the end, and trims the file if it has outgrown the bound.
    pub fn append(&self, bytes: &[u8])
    {
        let Some(parent) = self.file.parent()
        else
        {
            return;
        };

        if create_dir_all(parent).is_err()
        {
            return;
        }

        if let Ok(mut handle) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)
        {
            let _ = handle.write_all(bytes);
        }

        self.trim();
    }

    /// The last [`KEEP_BYTES`] of it, or nothing if there is no file yet.
    pub fn load(&self) -> Vec<u8>
    {
        let Ok(mut handle) = File::open(&self.file)
        else
        {
            return Vec::new();
        };

        let length = handle.metadata().map(|meta| meta.len()).unwrap_or(0);

        if handle
            .seek(SeekFrom::Start(length.saturating_sub(KEEP_BYTES)))
            .is_err()
        {
            return Vec::new();
        }

        let mut bytes = Vec::new();
        let _ = handle.read_to_end(&mut bytes);
        bytes
    }

    /// Leaves nothing for the next start to replay.
    pub fn clear(&self)
    {
        let _ = std::fs::remove_file(&self.file);
    }

    /// Rewrites the file as its own tail, once it has grown past [`TRIM_AT`].
    ///
    /// Rewriting is the whole trim. A rotating pair of files would avoid reading
    /// a mebibyte back, but a rotation boundary is one more place for the stream
    /// to not survive a restart, and surviving a restart is the only thing this
    /// file is for.
    fn trim(&self)
    {
        let Ok(meta) = std::fs::metadata(&self.file)
        else
        {
            return;
        };

        if meta.len() <= TRIM_AT
        {
            return;
        }

        let tail = self.load();
        let _ = std::fs::write(&self.file, tail);
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn scratch(name: &str) -> Scrollback
    {
        let directory = PathBuf::from("/private/tmp/trdr-t")
            .join(format!("scrollback-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);

        Scrollback::at(directory.join("run/terminal/scrollback.bin"))
    }

    /// The property the whole module is for: what comes back is what went in,
    /// escape bytes and invalid UTF-8 included. Anything that decoded the stream
    /// on the way through would fail here.
    #[test]
    fn round_trips_the_exact_bytes_including_escapes_and_invalid_utf8()
    {
        let scrollback = scratch("roundtrip");
        let written: Vec<u8> = vec![
            0x1b, b'[', b'3', b'1', b'm', b'h', b'i', 0x1b, b'[', b'0', b'm', 0xff, 0xfe, b'\n',
        ];

        scrollback.append(&written);

        assert_eq!(scrollback.load(), written);
    }

    #[test]
    fn appends_across_separate_writes_the_way_a_restart_reads_them()
    {
        let scrollback = scratch("append");

        scrollback.append(b"first ");
        scrollback.append(b"second");

        assert_eq!(scrollback.load(), b"first second".to_vec());
    }

    /// At the bound the oldest bytes go. The tail is what describes the screen
    /// as it is now, so it is the part that has to be there.
    #[test]
    fn trims_to_the_tail_once_the_file_grows_past_the_limit()
    {
        let scrollback = scratch("trim");

        scrollback.append(&vec![b'o'; (TRIM_AT + 1) as usize]);
        scrollback.append(b"tail");

        let kept = scrollback.load();

        assert!(kept.len() as u64 <= KEEP_BYTES);
        assert!(kept.ends_with(b"tail"));
    }

    #[test]
    fn clearing_leaves_nothing_for_the_next_start_to_replay()
    {
        let scrollback = scratch("clear");

        scrollback.append(b"gone");
        scrollback.clear();

        assert!(scrollback.load().is_empty());
    }

    #[test]
    fn a_missing_file_loads_as_empty_rather_than_failing()
    {
        assert!(scratch("missing").load().is_empty());
    }

    /// A path that cannot be created is not a reason to stop the terminal.
    #[test]
    fn an_unwritable_path_is_survived_rather_than_reported()
    {
        let scrollback = Scrollback::at("/dev/null/nowhere/scrollback.bin");

        scrollback.append(b"dropped");

        assert!(scrollback.load().is_empty());
    }
}
