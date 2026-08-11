//! SQLite: the canonical state of a workspace.
//!
//! What lands here (`docs/FOUNDATION_DESIGN.md` section 6): a bundled, patched
//! SQLite rather than the system one, WAL mode, foreign keys on, a startup
//! integrity check, ordered migrations that run only under the writer lease, and
//! the lease itself — the thing that keeps the app and the CLI from both writing.
//!
//! Two boundaries this module has to hold: no SQL and no connection ever reaches
//! the WebView, and append-only entities get no update or delete path in the
//! repository API at all.
//!
//! Nothing is implemented yet.
