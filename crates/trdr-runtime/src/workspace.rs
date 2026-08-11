//! The workspace directory on disk, and the `workspace.json` that identifies it.
//!
//! Section 5.1 of `docs/FOUNDATION_DESIGN.md` says a first run creates
//! `~/.trdr/workspaces/default` and puts a manifest in it: a portable workspace
//! id, the manifest's own schema version, and when the workspace was made. The
//! shape of that file is [`WorkspaceManifest`], which lives in `trdr-core`
//! because it is a value; creating one needs a clock and an id generator, which
//! is why the reading and writing live here.
//!
//! # Why this takes the lease
//!
//! [`Workspace::open_default`] asks for a [`WriterLease`] for the same reason
//! [`crate::db::Database::open`] does. A first run writes a file that decides
//! what this workspace *is*, and two processes doing that at once would leave
//! one of them holding an id the file no longer carries. Taking the lease is the
//! only way to get the argument, so the race cannot be written down.
//!
//! # What a first run does not create
//!
//! `~/.trdr/config.json` — the selected workspace and the per-machine instance
//! id Keychain items hang off — is section 5.1's other file and is not written
//! here. It belongs to the credential track, and inventing a local instance id
//! before anything stores a credential under it would be a value with no
//! meaning behind it.

use crate::clock::Clock;
use crate::ids::IdGenerator;
use crate::root::{RootError, WriterLease, PRIVATE_FILE_MODE};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use trdr_core::id::WorkspaceId;
use trdr_core::workspace::{ManifestError, WorkspaceManifest};

/// The file that says which workspace a directory is (section 5.1).
pub const MANIFEST_FILE_NAME: &str = "workspace.json";

/// One workspace directory, opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace
{
    path: PathBuf,
    manifest: WorkspaceManifest
}

impl Workspace
{
    /// Opens the lease holder's default workspace, creating it on a first run.
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use trdr_runtime::clock::SystemClock;
    /// # use trdr_runtime::ids::UlidGenerator;
    /// # use trdr_runtime::test_support::scratch_root;
    /// # use trdr_runtime::workspace::Workspace;
    /// let root = scratch_root("doc-workspace");
    /// let lease = root.acquire_writer_lease().unwrap();
    ///
    /// let first = Workspace::open_default(&lease, &SystemClock, &UlidGenerator).unwrap();
    /// let again = Workspace::open_default(&lease, &SystemClock, &UlidGenerator).unwrap();
    ///
    /// // A second open reads the manifest rather than minting a second identity.
    /// assert_eq!(first.id(), again.id());
    /// ```
    pub fn open_default(
        lease: &WriterLease,
        clock: &dyn Clock,
        ids: &dyn IdGenerator
    ) -> Result<Self, WorkspaceError>
    {
        let root = lease.root();
        root.prepare()?;

        let path = root.default_workspace_dir();
        let manifest_path = path.join(MANIFEST_FILE_NAME);

        let manifest = match fs::read_to_string(&manifest_path)
        {
            Ok(text) =>
            {
                WorkspaceManifest::from_json(&text).map_err(|source| WorkspaceError::Manifest {
                    path: manifest_path.clone(),
                    source
                })?
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound =>
            {
                let manifest =
                    WorkspaceManifest::new(WorkspaceId::from_ulid(ids.generate()), clock.now());
                write_manifest(&manifest_path, &manifest)?;
                manifest
            }
            Err(source) =>
            {
                return Err(WorkspaceError::Read {
                    path: manifest_path,
                    source
                })
            }
        };

        Ok(Self { path, manifest })
    }

    /// Where the workspace directory is.
    pub fn path(&self) -> &Path
    {
        &self.path
    }

    /// Where its manifest is.
    pub fn manifest_path(&self) -> PathBuf
    {
        self.path.join(MANIFEST_FILE_NAME)
    }

    /// The portable identity in that manifest.
    pub fn id(&self) -> WorkspaceId
    {
        self.manifest.workspace_id
    }

    /// The manifest itself.
    pub fn manifest(&self) -> &WorkspaceManifest
    {
        &self.manifest
    }
}

/// Writes a manifest so that no reader can ever see half of one.
///
/// The bytes go to a temporary file beside the destination and are renamed over
/// it, because `rename` within a directory is atomic: a reader sees either the
/// old name or the new contents, never a file that is being filled in. The
/// temporary name carries the process id so that a crashed run leaves something
/// identifiable rather than colliding with the next one.
fn write_manifest(path: &Path, manifest: &WorkspaceManifest) -> Result<(), WorkspaceError>
{
    let json = serde_json::to_vec(manifest).map_err(|_| WorkspaceError::Unserialisable)?;

    let directory = path.parent().unwrap_or(Path::new("."));
    let staging = directory.join(format!(".{MANIFEST_FILE_NAME}.{}.tmp", std::process::id()));

    let write = || -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(PRIVATE_FILE_MODE)
            .open(&staging)?;

        file.write_all(&json)?;
        // Before the rename, not after: a rename that lands before the contents
        // reach the disk is exactly the empty-file-after-a-crash case the
        // staging file was supposed to prevent.
        file.sync_all()?;
        drop(file);

        fs::rename(&staging, path)
    };

    write().map_err(|source| {
        let _ = fs::remove_file(&staging);
        WorkspaceError::Write {
            path: path.to_path_buf(),
            source
        }
    })
}

/// The workspace could not be opened or created.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError
{
    /// The directories under the product root could not be prepared.
    #[error(transparent)]
    Root(#[from] RootError),
    /// The manifest exists and could not be read off the disk.
    #[error("{path} could not be read: {source}")]
    Read
    {
        /// The manifest.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: io::Error
    },
    /// The manifest was read and is not one this build can use.
    #[error("{path} is not a workspace manifest this build can use: {source}")]
    Manifest
    {
        /// The manifest.
        path: PathBuf,
        /// Which rule it broke.
        #[source]
        source: ManifestError
    },
    /// The manifest could not be written.
    #[error("{path} could not be written: {source}")]
    Write
    {
        /// The manifest.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: io::Error
    },
    /// A manifest this build built could not be turned into JSON.
    ///
    /// Not reachable from any value [`WorkspaceManifest::new`] produces; it is
    /// here so that serialisation has an outcome rather than an `unwrap`.
    #[error("the workspace manifest could not be written as JSON")]
    Unserialisable
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::clock::SystemClock;
    use crate::ids::UlidGenerator;
    use crate::root::mode_of;
    use crate::test_support::{scratch_root, CountingIds, FixedClock};
    use trdr_core::workspace::WorkspaceSchemaVersion;

    #[test]
    fn a_first_run_creates_the_manifest_the_design_describes()
    {
        let root = scratch_root("ws-first-run");
        let lease = root.acquire_writer_lease().expect("the lease is free");
        let clock = FixedClock::at("2026-08-11T09:30:00Z");
        let ids = CountingIds::from(7);

        let workspace =
            Workspace::open_default(&lease, &clock, &ids).expect("a first run should create one");

        assert_eq!(workspace.path(), root.default_workspace_dir());
        assert_eq!(
            workspace.manifest().schema_version,
            WorkspaceSchemaVersion::V1
        );
        assert_eq!(
            workspace.manifest().created_at.as_str(),
            "2026-08-11T09:30:00Z"
        );

        // The file on disk, not the value in memory: what is asserted is that
        // the manifest was actually written, and written as itself.
        let text = fs::read_to_string(workspace.manifest_path()).expect("the manifest is there");
        assert_eq!(
            WorkspaceManifest::from_json(&text).expect("re-readable"),
            *workspace.manifest()
        );
    }

    #[test]
    fn the_manifest_is_private_to_its_owner()
    {
        let root = scratch_root("ws-modes");
        let lease = root.acquire_writer_lease().expect("the lease is free");
        let workspace =
            Workspace::open_default(&lease, &SystemClock, &UlidGenerator).expect("created");

        assert_eq!(
            mode_of(&workspace.manifest_path()).unwrap(),
            PRIVATE_FILE_MODE
        );
        assert_eq!(mode_of(workspace.path()).unwrap(), 0o700);
    }

    /// The property that makes a workspace id worth having: it is minted once
    /// and read every time after that. A second start that minted a new id would
    /// silently disown every backup taken under the old one.
    #[test]
    fn a_second_start_reads_the_identity_rather_than_minting_another()
    {
        let root = scratch_root("ws-stable-id");
        let lease = root.acquire_writer_lease().expect("the lease is free");
        let ids = CountingIds::from(1);

        let first = Workspace::open_default(&lease, &SystemClock, &ids).expect("created");
        let second = Workspace::open_default(&lease, &SystemClock, &ids).expect("reopened");

        assert_eq!(first.id(), second.id());
        assert_eq!(first.manifest(), second.manifest());
        assert_eq!(ids.issued(), 1, "the second start minted an id");
    }

    #[test]
    fn a_manifest_from_a_newer_build_is_told_apart_from_a_damaged_one()
    {
        let root = scratch_root("ws-newer");
        let lease = root.acquire_writer_lease().expect("the lease is free");
        root.prepare().unwrap();
        let path = root.default_workspace_dir().join(MANIFEST_FILE_NAME);

        fs::write(
            &path,
            "{\"schema_version\":\"trdr.workspace/v2\",\
             \"workspace_id\":\"01KZNNR5X818P3J6ENYKSADP8W\",\
             \"created_at\":\"2026-08-10T10:00:00Z\"}"
        )
        .unwrap();

        assert!(matches!(
            Workspace::open_default(&lease, &SystemClock, &UlidGenerator),
            Err(WorkspaceError::Manifest {
                source: ManifestError::UnsupportedSchemaVersion { .. },
                ..
            })
        ));

        fs::write(&path, "{ half a file").unwrap();

        assert!(matches!(
            Workspace::open_default(&lease, &SystemClock, &UlidGenerator),
            Err(WorkspaceError::Manifest {
                source: ManifestError::Unreadable,
                ..
            })
        ));
    }

    /// The staging file is an implementation detail, and it is one that would be
    /// noticed by a person opening the workspace in Finder. It must not survive
    /// a successful write.
    #[test]
    fn writing_the_manifest_leaves_nothing_beside_it()
    {
        let root = scratch_root("ws-no-litter");
        let lease = root.acquire_writer_lease().expect("the lease is free");
        let workspace =
            Workspace::open_default(&lease, &SystemClock, &UlidGenerator).expect("created");

        let names: Vec<String> = fs::read_dir(workspace.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();

        assert_eq!(names, [MANIFEST_FILE_NAME]);
    }
}
