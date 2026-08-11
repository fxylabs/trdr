//! The Unix socket the `trdr` CLI connects to.
//!
//! What lands here (section 9.2): the listener at `~/.trdr/run/app.sock` with
//! user-only permissions on both the directory and the file, the macOS peer uid
//! check, newline-delimited framing, and dispatch of the parsed frames that
//! `trdr_core::socket` defines.
//!
//! Parsing and refusing belong to the core; this module carries the frames and
//! decides nothing about what a method means.
//!
//! Nothing is implemented yet.
