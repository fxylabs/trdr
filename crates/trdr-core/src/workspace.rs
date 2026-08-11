//! `workspace.json`, the file that says which workspace this directory is.
//!
//! Section 5.1 of `docs/FOUNDATION_DESIGN.md` puts three things in it: the
//! portable workspace id, a schema version, and when it was created. The id is
//! portable on purpose — it travels inside a backup, so a restored copy on
//! another Mac is recognisably the same workspace. The per-machine identity that
//! Keychain items hang off is a different value, kept in `~/.trdr/config.json`,
//! and is deliberately not in this file: that is what stops a restored copy from
//! inheriting another machine's stored credentials by name.
//!
//! Nothing here reads or writes the file. Locating it, opening it, and taking
//! the writer lease are the runtime's job.

use crate::id::WorkspaceId;
use crate::time::Timestamp;
use serde::{Deserialize, Serialize};

/// The schema version this build writes into a new `workspace.json`.
pub const WORKSPACE_SCHEMA_V1: &str = "trdr.workspace/v1";

/// Which version of the workspace file this is.
///
/// A separate version from the database schema, the ingest bundle, and the
/// backup package, as section 6.1 requires. They move independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum WorkspaceSchemaVersion
{
    /// The first version.
    #[serde(rename = "trdr.workspace/v1")]
    V1
}

/// The contents of `workspace.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WorkspaceManifest
{
    /// Which version of this file's own schema is in use.
    pub schema_version: WorkspaceSchemaVersion,
    /// The portable identity of this workspace.
    pub workspace_id: WorkspaceId,
    /// When the workspace was created.
    pub created_at: Timestamp
}

impl WorkspaceManifest
{
    /// A manifest for a workspace being created now.
    pub fn new(workspace_id: WorkspaceId, created_at: Timestamp) -> Self
    {
        Self {
            schema_version: WorkspaceSchemaVersion::V1,
            workspace_id,
            created_at
        }
    }

    /// Reads a manifest, telling an unreadable file apart from one written by a
    /// version this build does not know.
    ///
    /// The difference matters to the person holding the workspace: the first is
    /// damage, the second is a newer trdr, and section 6.1 forbids opening
    /// something newer rather than guessing at it.
    pub fn from_json(text: &str) -> Result<Self, ManifestError>
    {
        #[derive(Deserialize)]
        struct SchemaProbe
        {
            schema_version: String
        }

        let probe: SchemaProbe =
            serde_json::from_str(text).map_err(|_| ManifestError::Unreadable)?;

        if probe.schema_version != WORKSPACE_SCHEMA_V1
        {
            return Err(ManifestError::UnsupportedSchemaVersion {
                found: probe.schema_version
            });
        }

        serde_json::from_str(text).map_err(|_| ManifestError::Unreadable)
    }
}

/// `workspace.json` could not be taken at face value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError
{
    /// The file is not a workspace manifest, or one of its values is not valid.
    #[error("the workspace manifest could not be read")]
    Unreadable,
    /// The file names a schema version this build does not know.
    #[error("workspace schema version {found} is not one this build knows")]
    UnsupportedSchemaVersion
    {
        /// The version the file claimed.
        found: String
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn sample() -> WorkspaceManifest
    {
        WorkspaceManifest::new(
            "01KZNNR5X818P3J6ENYKSADP8W".parse().unwrap(),
            Timestamp::parse("2026-08-10T10:00:00Z").unwrap()
        )
    }

    #[test]
    fn a_manifest_round_trips()
    {
        let json = serde_json::to_string(&sample()).unwrap();
        assert_eq!(
            json,
            "{\"schema_version\":\"trdr.workspace/v1\",\
             \"workspace_id\":\"01KZNNR5X818P3J6ENYKSADP8W\",\
             \"created_at\":\"2026-08-10T10:00:00Z\"}"
        );
        assert_eq!(WorkspaceManifest::from_json(&json).unwrap(), sample());
    }

    #[test]
    fn a_newer_schema_version_is_told_apart_from_damage()
    {
        let newer = "{\"schema_version\":\"trdr.workspace/v2\",\
                     \"workspace_id\":\"01KZNNR5X818P3J6ENYKSADP8W\",\
                     \"created_at\":\"2026-08-10T10:00:00Z\"}";
        assert_eq!(
            WorkspaceManifest::from_json(newer),
            Err(ManifestError::UnsupportedSchemaVersion {
                found: "trdr.workspace/v2".to_owned()
            })
        );
        assert_eq!(
            WorkspaceManifest::from_json("{"),
            Err(ManifestError::Unreadable)
        );
    }

    #[test]
    fn a_manifest_with_an_unusable_value_is_refused()
    {
        for json in [
            // A workspace id that is not a ULID.
            "{\"schema_version\":\"trdr.workspace/v1\",\"workspace_id\":\"default\",\
              \"created_at\":\"2026-08-10T10:00:00Z\"}",
            // A timestamp with a local offset.
            "{\"schema_version\":\"trdr.workspace/v1\",\
              \"workspace_id\":\"01KZNNR5X818P3J6ENYKSADP8W\",\
              \"created_at\":\"2026-08-10T19:00:00+09:00\"}",
            // A field this build does not know.
            "{\"schema_version\":\"trdr.workspace/v1\",\
              \"workspace_id\":\"01KZNNR5X818P3J6ENYKSADP8W\",\
              \"created_at\":\"2026-08-10T10:00:00Z\",\"local_instance_id\":\"x\"}"
        ]
        {
            assert_eq!(
                WorkspaceManifest::from_json(json),
                Err(ManifestError::Unreadable)
            );
        }
    }
}
