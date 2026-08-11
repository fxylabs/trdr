//! Scratch product roots and the stand-ins for the seams section 13 names.
//!
//! # Scratch roots
//!
//! [`scratch_root`] exists for one reason that is easy to get wrong. A macOS
//! `sockaddr_un` has 104 bytes for its path, and the per-user temporary
//! directory `std::env::temp_dir` hands back looks like
//! `/var/folders/9k/2b1n.../T/`, which is most of that budget before a test has
//! named anything. A socket bound under it fails with an error about the address
//! rather than about the length, and the failure only appears on the machine
//! whose temporary path happens to be long. Roots made here live directly under
//! `/private/tmp`, so the whole path stays around forty bytes.
//!
//! # Doubles
//!
//! [`FixedClock`] and [`CountingIds`] are the other half of the [`crate::clock`]
//! and [`crate::ids`] seams. A workspace manifest is two values a test cannot
//! predict — an instant and a random id — and these are what turn it into a file
//! whose exact bytes an assertion can name.
//!
//! Everything here is public so that the integration tests of this crate, of
//! `trdr-cli`, and of the desktop app share one definition rather than four
//! copies. None of it is part of the product, and nothing outside a test should
//! call it.

use crate::clock::Clock;
use crate::ids::IdGenerator;
use crate::root::ProductRoot;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use trdr_core::id::Ulid;
use trdr_core::time::Timestamp;

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

/// A clock stopped at one instant.
#[derive(Debug, Clone)]
pub struct FixedClock
{
    at: Timestamp
}

impl FixedClock
{
    /// A clock that always answers with this instant.
    ///
    /// # Panics
    ///
    /// If the text is not a canonical timestamp. A test that wrote one wrong
    /// should hear about it where it wrote it.
    pub fn at(text: &str) -> Self
    {
        Self {
            at: Timestamp::parse(text).expect("a fixed clock needs a canonical timestamp")
        }
    }
}

impl Clock for FixedClock
{
    fn now(&self) -> Timestamp
    {
        self.at.clone()
    }
}

/// An id generator that counts, so a test knows what it produced.
///
/// The values are ULIDs by type and by text, and they are deliberately not ULIDs
/// by construction: nothing here reads a clock, so `CountingIds::from(1)` gives
/// the same 26 characters on every machine and in every run.
#[derive(Debug)]
pub struct CountingIds
{
    first: u64,
    next: AtomicU64
}

impl CountingIds
{
    /// A generator whose first id is this number.
    pub fn from(first: u64) -> Self
    {
        Self {
            first,
            next: AtomicU64::new(first)
        }
    }

    /// How many ids have been handed out since it was made.
    pub fn issued(&self) -> u64
    {
        self.next.load(Ordering::Relaxed) - self.first
    }
}

impl IdGenerator for CountingIds
{
    fn generate(&self) -> Ulid
    {
        Ulid(u128::from(self.next.fetch_add(1, Ordering::Relaxed)))
    }
}
