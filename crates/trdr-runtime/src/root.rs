//! Where trdr keeps its state on disk, and the lease that makes one process the
//! writer.
//!
//! Section 5.1 of `docs/FOUNDATION_DESIGN.md` fixes the layout. The product root
//! is `~/.trdr` in production, but every path in this module hangs off a
//! [`ProductRoot`] value that the caller supplies, so a test never has to write
//! into the directory a person is actually using.
//!
//! ```text
//! <product root>/
//!   run/
//!     app.sock        the socket the CLI talks to
//!     writer.lock     the single writer lease
//!   workspaces/
//!     default/
//!       trdr.sqlite3  canonical state
//! ```
//!
//! The lease is the part worth reading carefully. It is an advisory `flock` held
//! on an open descriptor, and it is deliberately not a pid written into a file.
//! A pid file cannot answer the only question that matters after a crash —
//! is the holder still alive — because the recorded pid is either stale or has
//! been handed to an unrelated process, and reading the file cannot tell those
//! apart. The kernel drops an `flock` when the descriptor closes, and a process
//! that dies closes every descriptor it held, so the answer comes from the
//! kernel rather than from a guess.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

/// The mode section 5.1 fixes for the product root and every directory in it.
pub const PRIVATE_DIR_MODE: u32 = 0o700;

/// The mode section 5.1 fixes for a state file.
pub const PRIVATE_FILE_MODE: u32 = 0o600;

/// The workspace a first run creates (section 15).
pub const DEFAULT_WORKSPACE_NAME: &str = "default";

/// The canonical database inside a workspace (section 5.1).
pub const DATABASE_FILE_NAME: &str = "trdr.sqlite3";

/// The directory trdr keeps its runtime state and workspaces in.
///
/// Every path this crate opens is derived from one of these, and the only way to
/// make one is to say where it is. Production says `~/.trdr` through
/// [`ProductRoot::for_current_user`]; a test says a scratch directory through
/// [`ProductRoot::at`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductRoot
{
    path: PathBuf
}

impl ProductRoot
{
    /// A product root at a path the caller chose.
    pub fn at(path: impl Into<PathBuf>) -> Self
    {
        Self { path: path.into() }
    }

    /// The production root, `~/.trdr`.
    ///
    /// The one function in this crate that reads a path out of the environment.
    /// Nothing else falls back to it, so a caller that forgot to say where its
    /// state lives gets a compile error rather than a live workspace.
    pub fn for_current_user() -> Result<Self, RootError>
    {
        match std::env::var_os("HOME")
        {
            Some(home) if !home.is_empty() => Ok(Self::at(PathBuf::from(home).join(".trdr"))),
            _ => Err(RootError::HomeUnknown)
        }
    }

    /// Where this root is.
    pub fn path(&self) -> &Path
    {
        &self.path
    }

    /// `run/`, which holds the socket and the lease.
    pub fn run_dir(&self) -> PathBuf
    {
        self.path.join("run")
    }

    /// `run/app.sock`.
    pub fn socket_path(&self) -> PathBuf
    {
        self.run_dir().join("app.sock")
    }

    /// `run/writer.lock`.
    pub fn lease_path(&self) -> PathBuf
    {
        self.run_dir().join("writer.lock")
    }

    /// `workspaces/`.
    pub fn workspaces_dir(&self) -> PathBuf
    {
        self.path.join("workspaces")
    }

    /// `workspaces/default/`, the workspace a first run creates.
    pub fn default_workspace_dir(&self) -> PathBuf
    {
        self.workspaces_dir().join(DEFAULT_WORKSPACE_NAME)
    }

    /// `workspaces/default/trdr.sqlite3`.
    pub fn default_database_path(&self) -> PathBuf
    {
        self.default_workspace_dir().join(DATABASE_FILE_NAME)
    }

    /// Creates the directories this root needs, each private to its owner.
    ///
    /// Safe to call on a root that already exists, and it is worth calling every
    /// time: `create_dir_all` succeeds on an existing directory without touching
    /// its mode, and the umask decides the mode of a new one, so a directory that
    /// was once readable by everyone would stay that way if the mode were only
    /// set at creation.
    pub fn prepare(&self) -> Result<(), RootError>
    {
        for directory in [
            self.path.clone(),
            self.run_dir(),
            self.workspaces_dir(),
            self.default_workspace_dir()
        ]
        {
            make_private_dir(&directory)?;
        }

        Ok(())
    }

    /// Takes the single writer lease, or says who has it.
    ///
    /// Holding the returned value is what makes this process the writer. Dropping
    /// it, or dying, releases the lease.
    pub fn acquire_writer_lease(&self) -> Result<WriterLease, LeaseError>
    {
        self.prepare()?;

        let path = self.lease_path();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(PRIVATE_FILE_MODE)
            .open(&path)
            .map_err(|source| LeaseError::Open {
                path: path.clone(),
                source
            })?;

        // `mode` above only applies to a file this call created, so a lock file
        // left behind by an older build with a looser mode is tightened here.
        fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).map_err(
            |source| LeaseError::Open {
                path: path.clone(),
                source
            }
        )?;

        // SAFETY: `file` owns a valid descriptor for the whole call, and `flock`
        // reads nothing through it. The lock rides on this open file description,
        // which is why `WriterLease` keeps the `File` rather than the path.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
        {
            let error = io::Error::last_os_error();

            return match error.kind()
            {
                io::ErrorKind::WouldBlock => Err(LeaseError::Held { path }),
                _ => Err(LeaseError::Open {
                    path,
                    source: error
                })
            };
        }

        Ok(WriterLease {
            file,
            root: self.clone()
        })
    }
}

/// Creates one directory, private to its owner, whether or not it existed.
fn make_private_dir(path: &Path) -> Result<(), RootError>
{
    fs::create_dir_all(path).map_err(|source| RootError::Prepare {
        path: path.to_path_buf(),
        source
    })?;

    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_DIR_MODE)).map_err(|source| {
        RootError::Prepare {
            path: path.to_path_buf(),
            source
        }
    })
}

/// The permission bits on a path, without the file type.
pub fn mode_of(path: &Path) -> io::Result<u32>
{
    Ok(fs::metadata(path)?.permissions().mode() & 0o777)
}

/// The single writer lease of sections 5.1 and 6, held for as long as this value
/// lives.
///
/// There is no way to build one except by taking it, which is what lets the rest
/// of the crate ask for a `&WriterLease` and know the caller really is the
/// writer.
#[derive(Debug)]
pub struct WriterLease
{
    file: File,
    root: ProductRoot
}

impl WriterLease
{
    /// The root this lease was taken against.
    pub fn root(&self) -> &ProductRoot
    {
        &self.root
    }

    /// The lock file the lease is held on.
    pub fn path(&self) -> PathBuf
    {
        self.root.lease_path()
    }
}

impl Drop for WriterLease
{
    fn drop(&mut self)
    {
        // Closing `file` would release the lock on its own. Unlocking first is
        // here to say so out loud, and to release it before any other descriptor
        // this process may hold on the same file is closed.
        //
        // SAFETY: the descriptor is still open and owned by `self.file`.
        unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
    }
}

/// The product root could not be found or prepared.
#[derive(Debug, thiserror::Error)]
pub enum RootError
{
    /// `HOME` was not set, so `~/.trdr` cannot be named.
    #[error("HOME is not set, so the product root cannot be located")]
    HomeUnknown,
    /// A directory could not be created or made private.
    #[error("{path} could not be prepared: {source}")]
    Prepare
    {
        /// The directory that failed.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: io::Error
    }
}

/// The writer lease could not be taken.
#[derive(Debug, thiserror::Error)]
pub enum LeaseError
{
    /// Another live process holds it.
    ///
    /// Not "the file exists" — the kernel was asked and said the lock is held.
    #[error("another trdr process holds the writer lease on {path}")]
    Held
    {
        /// The lock file.
        path: PathBuf
    },
    /// The lock file could not be opened or locked for another reason.
    #[error("the writer lease on {path} could not be taken: {source}")]
    Open
    {
        /// The lock file.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: io::Error
    },
    /// The directories the lease lives in could not be prepared.
    #[error(transparent)]
    Root(#[from] RootError)
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::test_support::scratch_root;

    #[test]
    fn every_runtime_path_hangs_off_the_root_it_was_given()
    {
        let root = ProductRoot::at("/somewhere/else");

        assert_eq!(root.path(), Path::new("/somewhere/else"));
        assert_eq!(
            root.socket_path(),
            Path::new("/somewhere/else/run/app.sock")
        );
        assert_eq!(
            root.lease_path(),
            Path::new("/somewhere/else/run/writer.lock")
        );
        assert_eq!(
            root.default_database_path(),
            Path::new("/somewhere/else/workspaces/default/trdr.sqlite3")
        );
    }

    #[test]
    fn preparing_a_root_leaves_every_directory_private_to_its_owner()
    {
        let root = scratch_root("root-modes");
        root.prepare().expect("the root should be preparable");

        for directory in [
            root.path().to_path_buf(),
            root.run_dir(),
            root.workspaces_dir(),
            root.default_workspace_dir()
        ]
        {
            assert_eq!(
                mode_of(&directory).expect("the directory should exist"),
                PRIVATE_DIR_MODE,
                "{} is not private",
                directory.display()
            );
        }
    }

    #[test]
    fn a_directory_that_was_left_open_is_tightened_on_the_next_start()
    {
        let root = scratch_root("root-tighten");
        fs::create_dir_all(root.run_dir()).expect("the run directory could not be made");
        fs::set_permissions(root.run_dir(), fs::Permissions::from_mode(0o755))
            .expect("the mode could not be loosened");

        root.prepare().expect("the root should be preparable");
        assert_eq!(mode_of(&root.run_dir()).unwrap(), PRIVATE_DIR_MODE);
    }

    #[test]
    fn a_lease_file_is_readable_only_by_its_owner()
    {
        let root = scratch_root("lease-mode");
        let lease = root
            .acquire_writer_lease()
            .expect("the lease should be free");

        assert_eq!(mode_of(&lease.path()).unwrap(), PRIVATE_FILE_MODE);
    }

    // The crash case, in the small. A released lease is takeable again, which is
    // the same kernel path a dying process takes when its descriptors close.
    #[test]
    fn a_released_lease_can_be_taken_again()
    {
        let root = scratch_root("lease-release");
        let first = root
            .acquire_writer_lease()
            .expect("the lease should be free");
        drop(first);

        root.acquire_writer_lease()
            .expect("a released lease should be takeable");
    }

    #[test]
    fn a_lease_taken_against_one_root_says_which_root_that_was()
    {
        let root = scratch_root("lease-root");
        let lease = root
            .acquire_writer_lease()
            .expect("the lease should be free");

        assert_eq!(lease.root(), &root);
    }

    // Exclusion between processes is what the lease is for, and `flock` is per
    // open file description rather than per process — a second open in this same
    // process is allowed to succeed. So the exclusion proof lives in
    // `tests/writer_lease.rs`, which spawns a real child, and this test only
    // records why it is not here.
    #[test]
    fn exclusion_is_proven_between_processes_and_not_within_one()
    {
        let root = scratch_root("lease-same-process");
        let _first = root
            .acquire_writer_lease()
            .expect("the lease should be free");

        // Whatever the second attempt in this process does, it says nothing about
        // exclusion; the assertion is only that it does not panic or hang.
        let _second = root.acquire_writer_lease();
    }
}
