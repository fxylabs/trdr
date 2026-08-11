//! Everything in trdr that touches the outside world.
//!
//! `docs/FOUNDATION_DESIGN.md` section 4 puts one direction on the dependencies:
//! the app and the CLI both go through this crate, this crate goes to
//! [`trdr_core`], and nothing goes back. The core decides what is true; the
//! runtime is where SQLite, the Keychain, the network, the terminal, and the
//! socket actually happen.
//!
//! The modules are named one per track, so that two tracks are never editing the
//! same file and the shape of the crate does not have to be argued about again:
//!
//! | module | what lands there | design section |
//! |---|---|---|
//! | [`root`] | the product root, its layout, and the writer lease | 5.1, 6 |
//! | [`db`] | bundled SQLite and its migrations | 6 |
//! | [`keychain`] | credential handles, never secret values | 5.1, 8.3 |
//! | [`pty`] | the agent child process and its byte stream | 3 |
//! | [`socket`] | the Unix socket server the CLI talks to | 9.2 |
//! | [`collectors`] | the KIS, OpenDART, and ECOS collectors | 8.3 |
//!
//! Two rules hold across all of them, from section 3.1: this crate never writes
//! user-facing wording, and it never interprets what the agent printed.
//!
//! # Starting a runtime
//!
//! The order is not a convention that has to be remembered. Taking the writer
//! lease produces a value, and the database asks for it, so there is no way to
//! express them in the wrong order:
//!
//! ```no_run
//! use std::sync::Arc;
//! use trdr_runtime::db::Database;
//! use trdr_runtime::root::ProductRoot;
//! # fn start() -> Result<(), Box<dyn std::error::Error>> {
//! let root = ProductRoot::for_current_user()?;
//! let lease = Arc::new(root.acquire_writer_lease()?);
//!
//! let database = Database::open(Arc::clone(&lease))?;
//! # let _ = database;
//! # Ok(())
//! # }
//! ```

#![deny(missing_docs)]

pub mod collectors;
pub mod db;
pub mod keychain;
pub mod pty;
pub mod root;
pub mod socket;
pub mod test_support;

// Re-exported so that everything downstream sees one version of the domain
// types rather than depending on trdr-core separately and drifting.
pub use trdr_core;
