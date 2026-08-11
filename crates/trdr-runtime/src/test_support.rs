//! Scratch product roots for tests, short enough to hold a Unix socket.
//!
//! This module exists for one reason that is easy to get wrong. A macOS
//! `sockaddr_un` has 104 bytes for its path, and the per-user temporary
//! directory `std::env::temp_dir` hands back looks like
//! `/var/folders/9k/2b1n.../T/`, which is most of that budget before a test has
//! named anything. A socket bound under it fails with an error about the address
//! rather than about the length, and the failure only appears on the machine
//! whose temporary path happens to be long. Roots made here live directly under
//! `/private/tmp`, so the whole path stays around forty bytes.
//!
//! It is public so that the integration tests of this crate and of `trdr-cli`
//! share one definition rather than three copies. It is not part of the product,
//! and nothing outside a test should call it.

use crate::root::ProductRoot;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

/// Where scratch roots go. Deliberately short; see the module documentation.
const SCRATCH_PARENT: &str = "/private/tmp/trdr-t";

/// Tells two roots made in the same process apart.
static COUNTER: AtomicU32 = AtomicU32::new(0);

/// An empty product root for a test to write into, removed if it was left over.
///
/// The name only has to be readable in a failure; the process id and a counter
/// are what keep two test binaries, or two tests in one binary, apart.
pub fn scratch_root(name: &str) -> ProductRoot
{
    let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path =
        PathBuf::from(SCRATCH_PARENT).join(format!("{name}-{}-{serial}", std::process::id()));

    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("a scratch root could not be made");

    // The guard the brief asks for, stated where every test goes through it: a
    // scratch root is never allowed to be inside the directory holding a person's
    // real workspace.
    if let Some(home) = std::env::var_os("HOME").filter(|home| !home.is_empty())
    {
        assert!(
            !path.starts_with(PathBuf::from(home)),
            "a scratch root must never be inside the home directory"
        );
    }

    ProductRoot::at(path)
}
