use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

// Scrollback that survives an app restart has to live outside the WebView, and
// it is stored as the bytes that arrived. Nothing here decodes, splits or reads
// the stream: the host owns the PTY but must not turn its output into product
// state, so persistence has to stay a byte copy.
//
// The path is a parameter rather than a constant so the tests can prove the
// round trip without writing into the user's real `~/.trdr`.
const KEEP_BYTES: u64 = 1024 * 1024;
const TRIM_AT: u64 = 4 * KEEP_BYTES;

pub fn default_path() -> Option<PathBuf>
{
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".trdr/run/spike-pty/scrollback.bin"))
}

pub fn append(file: &Path, bytes: &[u8])
{
    let Some(parent) = file.parent() else { return };
    if create_dir_all(parent).is_err()
    {
        return;
    }
    if let Ok(mut handle) = OpenOptions::new().create(true).append(true).open(file)
    {
        let _ = handle.write_all(bytes);
    }
    trim(file);
}

pub fn load(file: &Path) -> Vec<u8>
{
    let Ok(mut handle) = File::open(file) else { return Vec::new() };
    let length = handle.metadata().map(|meta| meta.len()).unwrap_or(0);
    if handle.seek(SeekFrom::Start(length.saturating_sub(KEEP_BYTES))).is_err()
    {
        return Vec::new();
    }
    let mut bytes = Vec::new();
    let _ = handle.read_to_end(&mut bytes);
    bytes
}

pub fn clear(file: &Path)
{
    let _ = std::fs::remove_file(file);
}

// Rewriting the tail is the whole trim. A rolling file would be less code to
// read than a size check on every write, but this spike has to prove the byte
// stream survives a restart, and a rotation boundary is one more place for it
// to not survive.
fn trim(file: &Path)
{
    let Ok(meta) = std::fs::metadata(file) else { return };
    if meta.len() <= TRIM_AT
    {
        return;
    }
    let tail = load(file);
    let _ = std::fs::write(file, tail);
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn scratch(name: &str) -> PathBuf
    {
        let dir = std::env::temp_dir().join(format!("trdr-spike-scrollback-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("run/scrollback.bin")
    }

    #[test]
    fn round_trips_the_exact_bytes_including_escapes_and_invalid_utf8()
    {
        let file = scratch("roundtrip");
        let written: Vec<u8> = vec![0x1b, b'[', b'3', b'1', b'm', b'h', b'i', 0x1b, b'[', b'0', b'm', 0xff, 0xfe, b'\n'];
        append(&file, &written);
        assert_eq!(load(&file), written);
    }

    #[test]
    fn appends_across_separate_writes_the_way_a_restart_reads_them()
    {
        let file = scratch("append");
        append(&file, b"first ");
        append(&file, b"second");
        assert_eq!(load(&file), b"first second".to_vec());
    }

    #[test]
    fn trims_to_the_tail_once_the_file_grows_past_the_limit()
    {
        let file = scratch("trim");
        append(&file, &vec![b'o'; (TRIM_AT + 1) as usize]);
        append(&file, b"tail");
        let kept = load(&file);
        assert!(kept.len() as u64 <= KEEP_BYTES);
        assert!(kept.ends_with(b"tail"));
    }

    #[test]
    fn clearing_leaves_nothing_for_the_next_start_to_replay()
    {
        let file = scratch("clear");
        append(&file, b"gone");
        clear(&file);
        assert!(load(&file).is_empty());
    }

    #[test]
    fn a_missing_file_loads_as_empty_rather_than_failing()
    {
        assert!(load(&scratch("missing")).is_empty());
    }
}
