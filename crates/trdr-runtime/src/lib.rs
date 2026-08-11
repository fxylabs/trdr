//! Everything in trdr that touches the outside world.
//!
//! `docs/FOUNDATION_DESIGN.md` section 4 puts one direction on the dependencies:
//! the app and the CLI both go through this crate, this crate goes to
//! [`trdr_core`], and nothing goes back. The core decides what is true; the
//! runtime is where SQLite, the Keychain, the network, the terminal, and the
//! socket actually happen.
//!
//! At stage 0 the crate is a shell. The modules below are empty and named, one
//! per later track, so that two tracks are never editing the same file and the
//! shape of the crate does not have to be argued about again:
//!
//! | module | what lands there | design section |
//! |---|---|---|
//! | [`db`] | bundled SQLite, migrations, the single writer lease | 6 |
//! | [`keychain`] | credential handles, never secret values | 5.1, 8.3 |
//! | [`pty`] | the agent child process and its byte stream | 3 |
//! | [`socket`] | the Unix socket server the CLI talks to | 9.2 |
//! | [`collectors`] | the KIS, OpenDART, and ECOS collectors | 8.3 |
//!
//! Two rules hold across all of them, from section 3.1: this crate never writes
//! user-facing wording, and it never interprets what the agent printed.

#![deny(missing_docs)]

pub mod collectors;
pub mod db;
pub mod keychain;
pub mod pty;
pub mod socket;

// Re-exported so that everything downstream sees one version of the domain
// types rather than depending on trdr-core separately and drifting.
pub use trdr_core;
