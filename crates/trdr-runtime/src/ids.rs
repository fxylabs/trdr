//! Minting identifiers, behind the seam section 13 calls `IdGenerator`.
//!
//! Every identity in trdr is a ULID, and `trdr-core` deliberately cannot make
//! one: it takes the `ulid` crate with default features off, which drops the
//! clock and the random source and leaves only the codec. So the domain can read
//! and compare an id, and this crate is where one comes from.
//!
//! The trait exists for the same reason the clock's does. A workspace id ends up
//! in a file a test wants to assert on, and an assertion against a value that is
//! different every run is not an assertion; [`crate::test_support::CountingIds`]
//! is the substitute that makes it one.

use trdr_core::id::Ulid;

/// Where a fresh identifier comes from.
pub trait IdGenerator: Send + Sync
{
    /// A ULID nothing else has been given.
    fn generate(&self) -> Ulid;
}

/// The real one: 48 bits of the system clock and 80 bits of randomness.
#[derive(Debug, Clone, Copy, Default)]
pub struct UlidGenerator;

impl IdGenerator for UlidGenerator
{
    fn generate(&self) -> Ulid
    {
        Ulid::generate()
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use std::collections::BTreeSet;
    use trdr_core::id::WorkspaceId;

    #[test]
    fn a_generated_id_is_one_the_domain_will_accept_back()
    {
        let id = WorkspaceId::from_ulid(UlidGenerator.generate());

        assert_eq!(
            id.to_string().parse::<WorkspaceId>().expect("re-readable"),
            id
        );
    }

    #[test]
    fn two_generated_ids_are_different_and_sort_by_when_they_were_made()
    {
        let ids: Vec<Ulid> = (0..1_000).map(|_| UlidGenerator.generate()).collect();
        let distinct: BTreeSet<u128> = ids.iter().map(|id| id.0).collect();

        assert_eq!(distinct.len(), ids.len(), "a generated id repeated");

        // The timestamp is the high 48 bits, so ids made in one loop are
        // non-decreasing when compared as text, which is the property a manifest
        // and a recovery point directory both rely on.
        let first = WorkspaceId::from_ulid(ids[0]).to_string();
        let last = WorkspaceId::from_ulid(ids[ids.len() - 1]).to_string();

        assert!(first[..10] <= last[..10], "{first} then {last}");
    }
}
