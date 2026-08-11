use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

/// The product root. `TRDR_SPIKE_HOME` moves it so a test never writes into the
/// real `~/.trdr`, which is a user's live workspace on this machine.
pub fn product_root() -> PathBuf
{
    if let Ok(overridden) = std::env::var("TRDR_SPIKE_HOME")
    {
        return PathBuf::from(overridden);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".trdr")
}

pub fn run_dir() -> io::Result<PathBuf>
{
    prepare_run_dir(&product_root())
}

pub fn prepare_run_dir(root: &Path) -> io::Result<PathBuf>
{
    let directory = root.join("run");
    fs::create_dir_all(&directory)?;
    // Section 5.1 fixes the product root and `run` at 0700. Creating it that way
    // is not enough on its own: `create_dir_all` leaves an already-existing
    // directory as it found it, and umask decides the mode of a new one.
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
    Ok(directory)
}

pub fn socket_path() -> io::Result<PathBuf>
{
    Ok(run_dir()?.join("app.sock"))
}

pub fn lease_path() -> io::Result<PathBuf>
{
    Ok(run_dir()?.join("writer.lock"))
}

/// The single-writer lease of section 5.1, held for as long as this value lives.
///
/// The lock is an advisory `flock` on an open descriptor, which is what makes it
/// survivable: the kernel drops it when the descriptor closes, and a process
/// that crashes closes every descriptor it held. A lease recorded as a pid in a
/// file cannot do this — after a crash the pid is either stale or, worse, has
/// been reused by an unrelated process, and neither case is distinguishable from
/// a live holder by reading the file.
pub struct WriterLease
{
    file: File,
    path: PathBuf
}

impl WriterLease
{
    pub fn path(&self) -> &Path
    {
        &self.path
    }
}

impl Drop for WriterLease
{
    fn drop(&mut self)
    {
        // Explicit only to say what closing the descriptor means. The lock would
        // be released by the drop of `file` either way.
        unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
    }
}

pub fn acquire_lease(path: &Path) -> io::Result<WriterLease>
{
    let file = OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600).open(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;

    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
    {
        return Err(io::Error::new(io::ErrorKind::WouldBlock, "another trdr process holds the writer lease"));
    }
    Ok(WriterLease { file, path: path.to_path_buf() })
}

/// Removes a socket file left behind by a process that is no longer running.
///
/// The lease decides this, not the socket file. Whoever holds the writer lease
/// is the only live app, so any socket file it finds was left by a dead one.
/// Probing the socket instead — connect, see whether anything answers — is the
/// usual approach and is worse: a socket whose owner is alive but wedged answers
/// nothing, and one whose inode was reused answers something else.
pub fn clear_stale_socket(_lease: &WriterLease, socket: &Path) -> io::Result<()>
{
    match fs::remove_file(socket)
    {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
    }
}

pub fn mode_of(path: &Path) -> io::Result<u32>
{
    Ok(fs::metadata(path)?.permissions().mode() & 0o777)
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn scratch(name: &str) -> PathBuf
    {
        let directory = std::env::temp_dir().join(format!("trdr-spike-cli-bridge-{name}"));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("the scratch directory could not be made");
        directory
    }

    #[test]
    fn a_lease_refuses_a_second_holder()
    {
        let path = scratch("lease-second").join("writer.lock");
        let first = acquire_lease(&path).expect("the first holder should get the lease");
        let second = acquire_lease(&path);
        assert!(second.is_err(), "a second holder took a lease that was already held");
        drop(first);
    }

    // The crash case. A process that dies closes its descriptors, which is the
    // same kernel path as dropping the lease here, so the next start is not
    // blocked by a lock nobody holds.
    #[test]
    fn a_lease_left_by_a_dead_holder_does_not_block_the_next_one()
    {
        let path = scratch("lease-dead").join("writer.lock");
        let first = acquire_lease(&path).expect("the first holder should get the lease");
        drop(first);
        acquire_lease(&path).expect("a released lease should be takeable");
    }

    #[test]
    fn a_lease_file_is_readable_only_by_its_owner()
    {
        let path = scratch("lease-mode").join("writer.lock");
        let lease = acquire_lease(&path).expect("the lease should be taken");
        assert_eq!(mode_of(lease.path()).expect("the lease file should exist"), 0o600);
    }

    #[test]
    fn a_socket_file_left_behind_by_a_dead_app_is_cleared()
    {
        let directory = scratch("stale-socket");
        let lease = acquire_lease(&directory.join("writer.lock")).expect("the lease should be taken");
        let socket = directory.join("app.sock");
        fs::write(&socket, b"").expect("the leftover socket file could not be made");
        clear_stale_socket(&lease, &socket).expect("a leftover socket file should be clearable");
        assert!(!socket.exists());
        clear_stale_socket(&lease, &socket).expect("clearing nothing should not be an error");
    }

    #[test]
    fn the_run_directory_and_the_product_root_are_private_to_their_owner()
    {
        let root = scratch("run-mode");
        let directory = prepare_run_dir(&root).expect("the run directory should be creatable");
        assert_eq!(mode_of(&directory).expect("the run directory should exist"), 0o700);
        assert_eq!(mode_of(&root).expect("the product root should exist"), 0o700);
    }

    // `create_dir_all` returns Ok on a directory that already exists and leaves
    // its mode alone, so a run directory that was once world-readable stays that
    // way unless the mode is set on every start.
    #[test]
    fn a_run_directory_that_was_left_open_is_tightened_on_the_next_start()
    {
        let root = scratch("run-tighten");
        fs::create_dir_all(root.join("run")).expect("the run directory could not be made");
        fs::set_permissions(root.join("run"), fs::Permissions::from_mode(0o755)).expect("the mode could not be set");
        let directory = prepare_run_dir(&root).expect("the run directory should be usable");
        assert_eq!(mode_of(&directory).expect("the run directory should exist"), 0o700);
    }
}
