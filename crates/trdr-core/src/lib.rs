//! The trdr domain core.
//!
//! What lives here is the part of trdr that has no opinion about screens,
//! databases, or the operating system: identity, the shape of an error, and the
//! two inter-process contracts described in `docs/FOUNDATION_DESIGN.md`
//! section 9. Everything in this crate is a value that can be computed, parsed,
//! and compared without touching the outside world.
//!
//! At this stage the crate defines contracts only. No command is handled here,
//! and no handler will ever live here — the runtime owns effects.
//!
//! # Module map
//!
//! | module | contract |
//! |---|---|
//! | [`envelope`] | the `v` field every versioned message carries |
//! | [`error`] | the error envelope and its stable code set (section 12) |
//! | [`id`] | ULID-backed identifiers, including the portable workspace id |
//! | [`socket`] | `trdr` CLI ↔ running app frames (section 9.2) |
//! | [`time`] | the one way an instant is written down (section 7.2) |
//! | [`ui`] | React WebView ↔ Tauri host commands (section 9.1) |
//! | [`workspace`] | the `workspace.json` manifest (section 5.1) |

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod envelope;
pub mod error;
pub mod id;
pub mod socket;
pub mod time;
pub mod ui;
pub mod workspace;

pub use envelope::{EnvelopeVersion, PROTOCOL_VERSION};
pub use error::{ErrorCode, ErrorEnvelope, ErrorFamily, ErrorParam, Retryability, UpstreamStatus};
pub use id::{
    ApprovalRequestId, CauseChainId, RequestId, ResourceId, ScopedPathHandle, WorkspaceId
};
pub use time::Timestamp;
pub use workspace::WorkspaceManifest;
