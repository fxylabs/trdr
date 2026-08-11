//! The three collectors trdr ships with.
//!
//! What lands here (section 8.3): KIS, OpenDART, and ECOS behind one interface —
//! metadata, test, plan, collect — each producing bundles that go through the
//! same canonical ingest path a user-written collector's bundle does.
//!
//! A collector never commits a row itself, its cursor advances only after the
//! transaction that used it succeeded, and the KIS account snapshot goes through
//! the sensitive-data normaliser rather than the public object store. There is
//! no KRX collector, and section 1 keeps it that way.
//!
//! Nothing is implemented yet.
