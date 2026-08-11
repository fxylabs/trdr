//! Reads the one synthetic ingest bundle in `fixtures/synthetic/`.
//!
//! Section 8.1 of `docs/FOUNDATION_DESIGN.md` fixes the shape of an ingest
//! bundle — a `bundle.json` manifest beside NDJSON record files — and section
//! 5.2 requires every persisted object to carry a SHA-256 and a source
//! manifest. This test is what makes that requirement real for the fixture: it
//! recomputes the digest of every record file from the bytes on disk and
//! compares it against the digest the manifest declares. A manifest whose
//! hashes nobody recomputes is decoration.
//!
//! What this test is not: the ingest pipeline. There is no schema crate, no
//! collector and no database here, and the record types below are local to this
//! file on purpose. When the ingest track lands its real types, this fixture is
//! the first thing they should be pointed at, and these local structs go away.
//!
//! Every value in the bundle is synthetic and derived from a rule stated in
//! `daily_bars_follow_the_documented_synthetic_rule`. The market is `SYN`, the
//! instruments are `SYN0001` and `SYN0002`, and the upstream host is under the
//! `.invalid` top-level domain that RFC 2606 reserves so it can never resolve.
//! No real venue, ticker, account or price appears in this repository.

use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use trdr_core::id::Ulid;
use trdr_core::Timestamp;

/// The trading days the fixture covers, in the order the generator emitted them.
const DAYS: [&str; 5] = [
    "2026-08-03",
    "2026-08-04",
    "2026-08-05",
    "2026-08-06",
    "2026-08-07"
];

/// The `bundle.json` manifest of section 8.1.
///
/// `deny_unknown_fields` mirrors the ingest rule that an unknown field is
/// refused rather than silently dropped, so a stray key in the fixture fails
/// here instead of being ignored.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest
{
    schema_version: String,
    bundle_id: String,
    source: Source,
    collector: Collector,
    collected_at: String,
    files: Vec<FileEntry>
}

/// Where the records came from, as the person who built the bundle declares it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source
{
    id: String,
    upstream_url: String,
    terms_url: String,
    use_basis: String
}

/// What produced the bundle.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Collector
{
    name: String,
    version: String
}

/// One record file, with the digest the manifest commits to.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileEntry
{
    entity: String,
    path: String,
    records: usize,
    sha256: String
}

/// One NDJSON line: the record envelope of section 8.1.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope
{
    entity: String,
    key: serde_json::Map<String, serde_json::Value>,
    observed_at: String,
    available_at: String,
    revision: u32,
    payload: serde_json::Map<String, serde_json::Value>
}

/// The bundle directory, resolved from this crate rather than the working directory.
fn bundle_dir() -> PathBuf
{
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic/ingest-bundle-v1")
}

/// Reads and parses `bundle.json`.
fn manifest() -> Manifest
{
    let path = bundle_dir().join("bundle.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));

    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is not a valid manifest: {error}", path.display()))
}

/// Reads one record file into parsed envelopes.
fn envelopes(entry: &FileEntry) -> Vec<Envelope>
{
    let path = bundle_dir().join(&entry.path);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));

    text.lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line).unwrap_or_else(|error| {
                panic!(
                    "{}:{} is not a valid envelope: {error}",
                    entry.path,
                    index + 1
                )
            })
        })
        .collect()
}

/// Reads the numeric suffix of a synthetic symbol, so `SYN0002` is instrument 2.
fn symbol_index(symbol: &str) -> u64
{
    symbol
        .strip_prefix("SYN")
        .and_then(|digits| digits.parse().ok())
        .unwrap_or_else(|| panic!("{symbol} is not a synthetic symbol"))
}

/// Reads one string field out of a record's key or payload.
fn field<'a>(map: &'a serde_json::Map<String, serde_json::Value>, name: &str) -> &'a str
{
    map.get(name)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("expected a string field {name}"))
}

#[test]
fn sha256_matches_the_published_test_vectors()
{
    // The three vectors from FIPS 180-4: empty, one block, and two blocks. They
    // are what licenses the digests this file computes for the fixture below.
    assert_eq!(
        sha256::hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256::hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256::hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn declared_file_digests_match_the_bytes_on_disk()
{
    let manifest = manifest();
    assert!(!manifest.files.is_empty(), "the manifest lists no files");

    for entry in &manifest.files
    {
        let path = bundle_dir().join(&entry.path);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let computed = sha256::hex(&bytes);

        assert_eq!(
            computed, entry.sha256,
            "{} hashes to {computed}, but bundle.json declares {}. Either the \
             records changed without the manifest being updated, or the \
             manifest is wrong; regenerate both together.",
            entry.path, entry.sha256
        );

        let lines = std::fs::read_to_string(&path)
            .expect("already read once")
            .lines()
            .count();
        assert_eq!(
            lines, entry.records,
            "{} record count disagrees",
            entry.path
        );
    }
}

#[test]
fn the_manifest_declares_a_synthetic_source()
{
    let manifest = manifest();

    assert_eq!(manifest.schema_version, "trdr.ingest/v1");
    Ulid::from_string(&manifest.bundle_id).expect("bundle_id is a ULID");
    Timestamp::parse(manifest.collected_at.clone()).expect("collected_at is canonical UTC");

    // Section 8.2 reserves `trdr.*` for built-in collectors, so anything this
    // repository ships as a fixture has to live in the `user.*` namespace.
    assert_eq!(manifest.source.id, "user.synthetic");
    assert_eq!(manifest.source.use_basis, "synthetic");
    assert_eq!(manifest.collector.name, "trdr-synthetic-fixture");
    assert_eq!(manifest.collector.version, "1.0.0");

    for url in [&manifest.source.upstream_url, &manifest.source.terms_url]
    {
        assert!(
            url.starts_with("https://fixtures.invalid/"),
            "{url} points somewhere that can resolve; a fixture must name a \
             host under the reserved .invalid domain"
        );
    }
}

#[test]
fn every_record_envelope_is_well_formed()
{
    let manifest = manifest();
    let mut seen = BTreeSet::new();

    for entry in &manifest.files
    {
        let records = envelopes(entry);
        assert_eq!(
            records.len(),
            entry.records,
            "{} record count disagrees",
            entry.path
        );

        for (index, record) in records.iter().enumerate()
        {
            let where_ = format!("{}:{}", entry.path, index + 1);

            assert_eq!(
                record.entity, entry.entity,
                "{where_} declares another entity"
            );
            assert!(record.revision >= 1, "{where_} has revision 0");
            assert!(!record.key.is_empty(), "{where_} has an empty natural key");
            assert!(!record.payload.is_empty(), "{where_} has an empty payload");

            // Section 7.2 wants one spelling per instant, and `Timestamp` is
            // the type that decides what that spelling is.
            Timestamp::parse(record.observed_at.clone())
                .unwrap_or_else(|error| panic!("{where_} observed_at: {error}"));
            Timestamp::parse(record.available_at.clone())
                .unwrap_or_else(|error| panic!("{where_} available_at: {error}"));

            // Same source, key and revision may appear once. A second copy is
            // the conflict case, not a fixture.
            let identity = format!(
                "{}|{}|{}",
                record.entity,
                serde_json::to_string(&record.key).expect("a key serialises"),
                record.revision
            );
            assert!(seen.insert(identity), "{where_} repeats a key and revision");
        }
    }
}

#[test]
fn daily_bars_follow_the_documented_synthetic_rule()
{
    // The rule, in full: for instrument `s` (1-based, from the symbol suffix)
    // on trading day `d` (1-based, from DAYS),
    //
    //     open   = 10000 * s + 100 * d
    //     high   = open + 250
    //     low    = open - 150
    //     close  = open + 50
    //     volume = 1000 * s * d
    //
    // Prices are KRW integers written as strings, per section 7.2. Anyone can
    // regenerate the file from this paragraph, which is the point: no value in
    // it was observed anywhere.
    let manifest = manifest();
    let entry = manifest
        .files
        .iter()
        .find(|entry| entry.entity == "daily_bar")
        .expect("the bundle carries daily bars");

    let records = envelopes(entry);
    let mut corrections = 0;

    for record in &records
    {
        let symbol = symbol_index(field(&record.key, "symbol"));
        let date = field(&record.key, "date");
        let day = DAYS
            .iter()
            .position(|candidate| *candidate == date)
            .unwrap_or_else(|| panic!("{date} is outside the fixture's trading days"))
            as u64
            + 1;

        assert_eq!(field(&record.key, "market"), "SYN");

        let open = 10000 * symbol + 100 * day;
        assert_eq!(field(&record.payload, "open"), open.to_string());
        assert_eq!(field(&record.payload, "high"), (open + 250).to_string());
        assert_eq!(field(&record.payload, "low"), (open - 150).to_string());
        assert_eq!(field(&record.payload, "close"), (open + 50).to_string());

        // The one revision-2 record restates its bar with a corrected volume,
        // so the fixture carries the append-a-new-revision case and not only
        // the happy path.
        let volume = 1000 * symbol * day;
        match record.revision
        {
            1 => assert_eq!(field(&record.payload, "volume"), volume.to_string()),
            2 =>
            {
                corrections += 1;
                assert_eq!(field(&record.payload, "volume"), (volume + 7).to_string());
            }
            other => panic!("revision {other} is not part of the fixture")
        }
    }

    assert_eq!(
        corrections, 1,
        "the fixture carries exactly one corrected bar"
    );
}

/// A SHA-256 for this test only.
///
/// It is here so that checking the fixture's digests costs `trdr-core` no
/// dependency at all — not even a dev-dependency, in a crate whose whole point
/// is a dependency boundary narrow enough to audit at a glance. It is proven
/// against the FIPS 180-4 vectors above.
///
/// It is not a production hash. The hash boundary of section 7.2 — input and
/// output hashes for a backtest, and content addressing for stored objects —
/// belongs to the runtime and must use a reviewed implementation.
mod sha256
{
    /// The first 32 bits of the fractional parts of the cube roots of the first
    /// 64 primes.
    const ROUND_CONSTANTS: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2
    ];

    /// The first 32 bits of the fractional parts of the square roots of the
    /// first eight primes.
    const INITIAL_STATE: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19
    ];

    /// The digest of `message`, lower-case hexadecimal.
    pub fn hex(message: &[u8]) -> String
    {
        digest(message)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    /// Pads the message and runs it through the compression function.
    fn digest(message: &[u8]) -> [u8; 32]
    {
        let bit_length = (message.len() as u64) * 8;
        let mut padded = message.to_vec();
        padded.push(0x80);

        while padded.len() % 64 != 56
        {
            padded.push(0);
        }

        padded.extend_from_slice(&bit_length.to_be_bytes());

        let mut state = INITIAL_STATE;

        for block in padded.chunks_exact(64)
        {
            compress(&mut state, block);
        }

        let mut digest = [0u8; 32];

        for (slot, word) in digest.chunks_exact_mut(4).zip(state.iter())
        {
            slot.copy_from_slice(&word.to_be_bytes());
        }

        digest
    }

    /// One 64-byte block, folded into the running state.
    fn compress(state: &mut [u32; 8], block: &[u8])
    {
        let mut schedule = [0u32; 64];

        for (slot, word) in schedule.iter_mut().zip(block.chunks_exact(4))
        {
            *slot = u32::from_be_bytes(word.try_into().expect("a four-byte word"));
        }

        for index in 16..64
        {
            let previous = schedule[index - 15];
            let recent = schedule[index - 2];
            let s0 = previous.rotate_right(7) ^ previous.rotate_right(18) ^ (previous >> 3);
            let s1 = recent.rotate_right(17) ^ recent.rotate_right(19) ^ (recent >> 10);

            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;

        for (constant, word) in ROUND_CONSTANTS.iter().zip(schedule.iter())
        {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(*constant)
                .wrapping_add(*word);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(majority);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h])
        {
            *slot = slot.wrapping_add(value);
        }
    }
}
