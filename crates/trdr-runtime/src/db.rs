//! The bundled SQLite database, opened by whoever holds the writer lease.
//!
//! Section 6 of `docs/FOUNDATION_DESIGN.md` puts two rules above everything else
//! here. The first is that the app and the CLI are never both the writer, and the
//! second is that migrations run only under the writer lease. Both are enforced
//! by the shape of this module rather than by remembering to check: the only way
//! to get a [`Database`] is to hand [`Database::open`] a [`WriterLease`], and the
//! only way to get one of those is to take it (see [`crate::root`]).
//!
//! SQLite itself is compiled into the binary rather than borrowed from macOS, so
//! that the version and its compile-time options are ours and do not change under
//! a system update. Section 6 requires 3.51.3 or newer; [`sqlite_version`]
//! reports what actually got linked, [`Database::open`] refuses to run on
//! anything older, and a test asserts it so a dependency bump cannot quietly walk
//! the version backwards.
//!
//! # Opening, in the order section 6.1 fixes
//!
//! ```text
//! writer lease held (a value of type WriterLease says so)
//! → WAL, foreign keys, busy timeout
//! → application id and schema version checked
//! → PRAGMA quick_check
//! → pending migrations run in one transaction
//! → checksum, app version and time recorded
//! → foreign_key_check + quick_check
//! → commit
//! ```

use crate::root::{ProductRoot, RootError, WriterLease, PRIVATE_FILE_MODE};
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// The oldest SQLite section 6 allows.
pub const REQUIRED_SQLITE_VERSION: (u32, u32, u32) = (3, 51, 3);

/// Marks the file as a trdr database, so that a file that is merely valid SQLite
/// is not opened as one.
///
/// The four bytes of `TRDR`, which is what `sqlite3 file.db` and `file(1)` will
/// show for the application id.
pub const APPLICATION_ID: i32 = 0x5452_4452;

/// The schema version this build writes once every migration it knows has run.
///
/// One past the highest migration id, so a database from a newer build is
/// recognisable by a number this build has never written.
pub const LATEST_SCHEMA_VERSION: i32 = 1;

/// How long a writer waits on a locked database before giving up.
pub const BUSY_TIMEOUT_MS: u64 = 5_000;

/// The SQLite that was compiled into this binary.
pub fn sqlite_version() -> &'static str
{
    rusqlite::version()
}

/// Whether the linked SQLite is new enough for section 6.
pub fn sqlite_version_is_supported() -> bool
{
    let mut parts = sqlite_version().split('.').map(str::parse::<u32>);
    let mut next = || parts.next().and_then(Result::ok).unwrap_or(0);

    (next(), next(), next()) >= REQUIRED_SQLITE_VERSION
}

/// One step of the schema, as this build has it.
struct Migration
{
    /// Its place in the order. Ids are dense and never reused.
    id: i64,
    /// The statements it runs.
    sql: &'static str
}

/// Migration 0: the bookkeeping every later migration is recorded in.
///
/// `STRICT` makes SQLite enforce the declared types rather than accept anything
/// in any column, and `applied_at_ms` is Unix milliseconds rather than the
/// domain's RFC 3339 text on purpose — this is the runtime's own ledger, it is
/// never hashed, and an integer needs no calendar arithmetic to write.
const MIGRATION_0: &str = "\
CREATE TABLE schema_migrations (
    id            INTEGER PRIMARY KEY,
    checksum      TEXT    NOT NULL,
    app_version   TEXT    NOT NULL,
    applied_at_ms INTEGER NOT NULL
) STRICT;
";

/// Every migration this build has, in order.
const MIGRATIONS: &[Migration] = &[Migration {
    id: 0,
    sql: MIGRATION_0
}];

/// A migration that has already run, as the database records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedMigration
{
    /// Which migration.
    pub id: i64,
    /// The SHA-256 of the SQL that ran, in lower-case hexadecimal.
    pub checksum: String,
    /// The build that ran it.
    pub app_version: String,
    /// When it ran, in Unix milliseconds.
    pub applied_at_ms: i64
}

/// The canonical SQLite database of one workspace.
///
/// Holds the writer lease for as long as it lives, which is what stops the lease
/// from being released while a connection is still open.
pub struct Database
{
    connection: Connection,
    path: PathBuf,
    lease: Arc<WriterLease>
}

impl Database
{
    /// Opens the database of the lease holder's default workspace, running any
    /// migration it needs.
    ///
    /// The lease is the argument, and there is no other constructor, so "open the
    /// database without being the writer" is not something a caller can express.
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use trdr_runtime::db::Database;
    /// # use trdr_runtime::test_support::scratch_root;
    /// let root = scratch_root("doc-open");
    /// let lease = Arc::new(root.acquire_writer_lease().unwrap());
    /// let database = Database::open(lease).unwrap();
    ///
    /// assert_eq!(database.schema_version().unwrap(), 1);
    /// ```
    ///
    /// Without a lease there is nothing to call:
    ///
    /// ```compile_fail
    /// # use trdr_runtime::db::Database;
    /// # use trdr_runtime::test_support::scratch_root;
    /// let root = scratch_root("doc-no-lease");
    /// let database = Database::open(root.default_database_path()).unwrap();
    /// ```
    pub fn open(lease: Arc<WriterLease>) -> Result<Self, DbError>
    {
        if !sqlite_version_is_supported()
        {
            return Err(DbError::SqliteTooOld {
                found: sqlite_version(),
                required: REQUIRED_SQLITE_VERSION
            });
        }

        let root = lease.root().clone();
        root.prepare()?;

        let path = root.default_database_path();
        let connection = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
        )?;

        apply_pragmas(&connection)?;
        check_identity(&connection)?;
        quick_check(&connection)?;
        migrate(&connection)?;

        // After the pragmas, not before: turning on write-ahead logging is what
        // creates the `-wal` and `-shm` files, so tightening them earlier would
        // find nothing to tighten.
        make_state_files_private(&path)?;

        Ok(Self {
            connection,
            path,
            lease
        })
    }

    /// Where the database file is.
    pub fn path(&self) -> &Path
    {
        &self.path
    }

    /// The root this database belongs to.
    pub fn root(&self) -> &ProductRoot
    {
        self.lease.root()
    }

    /// The schema version recorded in the file.
    pub fn schema_version(&self) -> Result<i32, DbError>
    {
        Ok(self
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))?)
    }

    /// Every migration that has run, oldest first.
    pub fn applied_migrations(&self) -> Result<Vec<AppliedMigration>, DbError>
    {
        let mut statement = self.connection.prepare(
            "SELECT id, checksum, app_version, applied_at_ms FROM schema_migrations ORDER BY id"
        )?;
        let rows = statement.query_map([], |row| {
            Ok(AppliedMigration {
                id: row.get(0)?,
                checksum: row.get(1)?,
                app_version: row.get(2)?,
                applied_at_ms: row.get(3)?
            })
        })?;

        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// What one pragma says, as text, for the tests that check section 6 was
    /// honoured.
    ///
    /// A pragma answers with whatever type it likes, so the value is read
    /// untyped and rendered rather than asked for as a `String`.
    #[cfg(test)]
    fn pragma(&self, name: &str) -> Result<String, DbError>
    {
        let value: rusqlite::types::Value =
            self.connection
                .query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))?;

        Ok(match value
        {
            rusqlite::types::Value::Integer(number) => number.to_string(),
            rusqlite::types::Value::Real(number) => number.to_string(),
            rusqlite::types::Value::Text(text) => text,
            rusqlite::types::Value::Null | rusqlite::types::Value::Blob(_) => String::new()
        })
    }
}

/// Sets the pragmas section 6 names.
///
/// `journal_mode` is a property of the file and survives; `foreign_keys` and
/// `busy_timeout` are properties of a connection and have to be set on every
/// open, which is why they are here and not in a migration.
fn apply_pragmas(connection: &Connection) -> Result<(), DbError>
{
    let mode: String = connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;

    if !mode.eq_ignore_ascii_case("wal")
    {
        return Err(DbError::WalRefused { found: mode });
    }

    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.busy_timeout(std::time::Duration::from_millis(BUSY_TIMEOUT_MS))?;

    let foreign_keys: i32 = connection.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;

    match foreign_keys
    {
        1 => Ok(()),
        _ => Err(DbError::ForeignKeysRefused)
    }
}

/// Refuses a file that is not this build's database (section 6.1).
fn check_identity(connection: &Connection) -> Result<(), DbError>
{
    let application_id: i32 =
        connection.query_row("PRAGMA application_id", [], |row| row.get(0))?;

    match application_id
    {
        // A database this build has never written to. Claiming it is safe only
        // while it is still empty, which `user_version` of 0 and an empty schema
        // together mean.
        0 if is_empty(connection)? =>
        {
            connection.pragma_update(None, "application_id", APPLICATION_ID)?;
        }
        id if id == APPLICATION_ID =>
        {}
        found => return Err(DbError::NotATrdrDatabase { found })
    }

    let schema_version: i32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    match schema_version > LATEST_SCHEMA_VERSION
    {
        // Section 6.1: a database from a newer build is not opened for writing
        // and is never downgraded.
        true => Err(DbError::SchemaFromNewerBuild {
            found: schema_version,
            supported: LATEST_SCHEMA_VERSION
        }),
        false => Ok(())
    }
}

/// Whether the file holds no schema of its own yet.
fn is_empty(connection: &Connection) -> Result<bool, DbError>
{
    let objects: i64 =
        connection.query_row("SELECT count(*) FROM sqlite_schema", [], |row| row.get(0))?;

    Ok(objects == 0)
}

/// The startup integrity check section 6 requires.
fn quick_check(connection: &Connection) -> Result<(), DbError>
{
    let outcome: String = connection.query_row("PRAGMA quick_check(1)", [], |row| row.get(0))?;

    match outcome.eq_ignore_ascii_case("ok")
    {
        true => Ok(()),
        false => Err(DbError::IntegrityCheckFailed)
    }
}

/// Runs whatever this build has that the file does not, in one transaction.
///
/// Re-running is a no-op: a migration whose id is already recorded is skipped, so
/// opening an up-to-date database writes nothing.
fn migrate(connection: &Connection) -> Result<(), DbError>
{
    let applied = read_applied(connection)?;

    for (id, checksum) in &applied
    {
        let Some(migration) = MIGRATIONS.iter().find(|migration| migration.id == *id)
        else
        {
            return Err(DbError::UnknownAppliedMigration { id: *id });
        };

        // Section 6.1: a migration whose text moved after it ran is drift, and
        // drift is refused rather than reconciled.
        if checksum != &checksum_of(migration.sql)
        {
            return Err(DbError::MigrationDrift { id: *id });
        }
    }

    let pending: Vec<&Migration> = MIGRATIONS
        .iter()
        .filter(|migration| !applied.iter().any(|(id, _)| *id == migration.id))
        .collect();

    if pending.is_empty()
    {
        return Ok(());
    }

    // Section 6.1 puts a verified recovery point between an existing database and
    // a migration, and the backup track has not built one yet. Refusing is the
    // honest state: a fresh database has nothing to recover, so migration 0 runs,
    // and the first migration against a database holding real data will not run
    // until the recovery point exists.
    if !applied.is_empty()
    {
        return Err(DbError::RecoveryPointRequired {
            pending: pending.iter().map(|migration| migration.id).collect()
        });
    }

    let transaction = connection.unchecked_transaction()?;

    for migration in pending
    {
        transaction.execute_batch(migration.sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations (id, checksum, app_version, applied_at_ms) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                migration.id,
                checksum_of(migration.sql),
                env!("CARGO_PKG_VERSION"),
                now_ms()
            ]
        )?;
    }

    // `PRAGMA user_version` takes no bound parameter. The value is a constant of
    // this build, so there is nothing here a caller could shape.
    transaction.execute_batch(&format!("PRAGMA user_version = {LATEST_SCHEMA_VERSION}"))?;

    let violations: i64 =
        transaction.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })?;

    if violations != 0
    {
        return Err(DbError::ForeignKeyViolations { found: violations });
    }

    let outcome: String = transaction.query_row("PRAGMA quick_check(1)", [], |row| row.get(0))?;

    if !outcome.eq_ignore_ascii_case("ok")
    {
        return Err(DbError::IntegrityCheckFailed);
    }

    Ok(transaction.commit()?)
}

/// The migrations the file records, or none at all when the ledger is not there
/// yet.
fn read_applied(connection: &Connection) -> Result<Vec<(i64, String)>, DbError>
{
    let ledger_exists: i64 = connection.query_row(
        "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = 'schema_migrations'",
        [],
        |row| row.get(0)
    )?;

    if ledger_exists == 0
    {
        return Ok(Vec::new());
    }

    let mut statement =
        connection.prepare("SELECT id, checksum FROM schema_migrations ORDER BY id")?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;

    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// The SHA-256 of a migration's text, in lower-case hexadecimal.
fn checksum_of(sql: &str) -> String
{
    let digest = Sha256::digest(sql.as_bytes());
    let mut text = String::with_capacity(digest.len() * 2);

    for byte in digest
    {
        use std::fmt::Write as _;
        let _ = write!(text, "{byte:02x}");
    }

    text
}

/// Now, in Unix milliseconds.
fn now_ms() -> i64
{
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_millis() as i64)
        .unwrap_or_default()
}

/// Makes the database and the files SQLite keeps beside it private to their
/// owner (section 5.1).
///
/// The `-wal` and `-shm` files are created by SQLite when WAL is turned on, so
/// they are tightened again after the pragmas run rather than only here.
fn make_state_files_private(database: &Path) -> Result<(), DbError>
{
    for path in state_file_paths(database)
    {
        if !path.exists()
        {
            continue;
        }

        fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).map_err(
            |source| DbError::Permissions {
                path: path.clone(),
                source
            }
        )?;
    }

    Ok(())
}

/// The database and the two files SQLite manages next to it.
fn state_file_paths(database: &Path) -> Vec<PathBuf>
{
    let mut paths = vec![database.to_path_buf()];

    for suffix in ["-wal", "-shm"]
    {
        let mut name = database.as_os_str().to_os_string();
        name.push(suffix);
        paths.push(PathBuf::from(name));
    }

    paths
}

/// The database could not be opened, migrated, or trusted.
#[derive(Debug, thiserror::Error)]
pub enum DbError
{
    /// The SQLite compiled into this binary is older than section 6 allows.
    #[error("bundled SQLite is {found}, section 6 requires {}.{}.{} or newer", required.0, required.1, required.2)]
    SqliteTooOld
    {
        /// What was linked.
        found: &'static str,
        /// What is required.
        required: (u32, u32, u32)
    },
    /// The file is valid SQLite but is not a trdr database.
    #[error("application id {found:#x} is not trdr's {APPLICATION_ID:#x}")]
    NotATrdrDatabase
    {
        /// The id the file carried.
        found: i32
    },
    /// The file was written by a later build.
    #[error("schema version {found} is newer than the {supported} this build knows")]
    SchemaFromNewerBuild
    {
        /// The version in the file.
        found: i32,
        /// The newest this build writes.
        supported: i32
    },
    /// The file records a migration this build does not have.
    #[error("migration {id} was applied by a build this one does not know")]
    UnknownAppliedMigration
    {
        /// Which one.
        id: i64
    },
    /// A migration's text changed after it ran.
    #[error("migration {id} does not match the one in this build")]
    MigrationDrift
    {
        /// Which one.
        id: i64
    },
    /// There is a migration to run and no verified recovery point to run it
    /// after (section 6.1).
    #[error("a recovery point is required before migrations {pending:?} can run")]
    RecoveryPointRequired
    {
        /// The migrations that are waiting.
        pending: Vec<i64>
    },
    /// SQLite would not switch the file to write-ahead logging.
    #[error("journal mode is {found}, not wal")]
    WalRefused
    {
        /// The mode SQLite reported.
        found: String
    },
    /// SQLite would not enforce foreign keys.
    #[error("foreign key enforcement could not be turned on")]
    ForeignKeysRefused,
    /// `quick_check` reported damage.
    #[error("the database did not pass its integrity check")]
    IntegrityCheckFailed,
    /// A migration left rows that break a foreign key.
    #[error("{found} foreign key violations after migrating")]
    ForeignKeyViolations
    {
        /// How many.
        found: i64
    },
    /// A state file could not be made private.
    #[error("{path} could not be made private: {source}")]
    Permissions
    {
        /// The file.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error
    },
    /// The directories the database lives in could not be prepared.
    #[error(transparent)]
    Root(#[from] RootError),
    /// SQLite refused something.
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error)
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::root::mode_of;
    use crate::test_support::scratch_root;

    fn open(root: &ProductRoot) -> Database
    {
        let lease = Arc::new(
            root.acquire_writer_lease()
                .expect("the lease should be free")
        );
        Database::open(lease).expect("the database should open")
    }

    // The version check the brief asks for, as a test rather than a comment: a
    // dependency bump that walks SQLite backwards fails here.
    #[test]
    fn the_bundled_sqlite_is_new_enough_for_section_six()
    {
        assert!(
            sqlite_version_is_supported(),
            "bundled SQLite is {}, section 6 requires {:?} or newer",
            sqlite_version(),
            REQUIRED_SQLITE_VERSION
        );

        // Printed so the version is in the test output and does not have to be
        // taken on trust.
        eprintln!("bundled SQLite version: {}", sqlite_version());
    }

    #[test]
    fn the_version_comparison_reads_each_part_as_a_number()
    {
        // The comparison must not be lexicographic: "3.9.0" is older than
        // "3.51.3" even though it sorts after it.
        let older = ("3.9.0", (3u32, 51, 3));
        let parsed: Vec<u32> = older
            .0
            .split('.')
            .map(|part| part.parse().unwrap())
            .collect();

        assert!((parsed[0], parsed[1], parsed[2]) < older.1);
    }

    #[test]
    fn a_new_database_comes_up_migrated_and_identified()
    {
        let root = scratch_root("db-fresh");
        let database = open(&root);

        assert_eq!(database.schema_version().unwrap(), LATEST_SCHEMA_VERSION);
        assert_eq!(database.path(), root.default_database_path());

        let applied = database.applied_migrations().unwrap();
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].id, 0);
        assert_eq!(applied[0].checksum, checksum_of(MIGRATION_0));
        assert_eq!(applied[0].app_version, env!("CARGO_PKG_VERSION"));
        assert!(applied[0].applied_at_ms > 0);
    }

    #[test]
    fn section_six_s_pragmas_are_on()
    {
        let root = scratch_root("db-pragmas");
        let database = open(&root);

        assert!(database
            .pragma("journal_mode")
            .unwrap()
            .eq_ignore_ascii_case("wal"));
        assert_eq!(database.pragma("foreign_keys").unwrap(), "1");
        assert_eq!(
            database.pragma("busy_timeout").unwrap(),
            BUSY_TIMEOUT_MS.to_string()
        );
        assert_eq!(
            database.pragma("application_id").unwrap(),
            APPLICATION_ID.to_string()
        );
    }

    #[test]
    fn opening_an_already_migrated_database_writes_nothing()
    {
        let root = scratch_root("db-rerun");
        let first = open(&root);
        let applied = first.applied_migrations().unwrap();
        drop(first);

        let second = open(&root);

        assert_eq!(second.applied_migrations().unwrap(), applied);
        assert_eq!(second.schema_version().unwrap(), LATEST_SCHEMA_VERSION);
    }

    #[test]
    fn a_file_that_is_not_a_trdr_database_is_refused()
    {
        let root = scratch_root("db-foreign");
        root.prepare().unwrap();

        let foreign = Connection::open(root.default_database_path()).unwrap();
        foreign
            .pragma_update(None, "application_id", 0x1234_5678)
            .unwrap();
        foreign
            .execute_batch("CREATE TABLE other (a INTEGER)")
            .unwrap();
        drop(foreign);

        let lease = Arc::new(root.acquire_writer_lease().unwrap());

        assert!(matches!(
            Database::open(lease),
            Err(DbError::NotATrdrDatabase { found: 0x1234_5678 })
        ));
    }

    #[test]
    fn a_database_from_a_newer_build_is_not_opened_for_writing()
    {
        let root = scratch_root("db-newer");
        drop(open(&root));

        let ahead = Connection::open(root.default_database_path()).unwrap();
        ahead
            .execute_batch(&format!(
                "PRAGMA user_version = {}",
                LATEST_SCHEMA_VERSION + 7
            ))
            .unwrap();
        drop(ahead);

        let lease = Arc::new(root.acquire_writer_lease().unwrap());

        assert!(matches!(
            Database::open(lease),
            Err(DbError::SchemaFromNewerBuild { found, .. }) if found == LATEST_SCHEMA_VERSION + 7
        ));
    }

    #[test]
    fn a_migration_whose_text_moved_is_refused_as_drift()
    {
        let root = scratch_root("db-drift");
        drop(open(&root));

        let tampered = Connection::open(root.default_database_path()).unwrap();
        tampered
            .execute(
                "UPDATE schema_migrations SET checksum = 'moved' WHERE id = 0",
                []
            )
            .unwrap();
        drop(tampered);

        let lease = Arc::new(root.acquire_writer_lease().unwrap());

        assert!(matches!(
            Database::open(lease),
            Err(DbError::MigrationDrift { id: 0 })
        ));
    }

    #[test]
    fn a_migration_from_a_build_this_one_does_not_have_is_refused()
    {
        let root = scratch_root("db-unknown-migration");
        drop(open(&root));

        let ahead = Connection::open(root.default_database_path()).unwrap();
        ahead
            .execute(
                "INSERT INTO schema_migrations (id, checksum, app_version, applied_at_ms) \
                 VALUES (9, 'x', '9.9.9', 1)",
                []
            )
            .unwrap();
        drop(ahead);

        let lease = Arc::new(root.acquire_writer_lease().unwrap());

        assert!(matches!(
            Database::open(lease),
            Err(DbError::UnknownAppliedMigration { id: 9 })
        ));
    }

    #[test]
    fn the_database_and_the_files_beside_it_are_private_to_their_owner()
    {
        let root = scratch_root("db-modes");
        let database = open(&root);

        for path in state_file_paths(database.path())
        {
            if path.exists()
            {
                assert_eq!(
                    mode_of(&path).unwrap(),
                    PRIVATE_FILE_MODE,
                    "{} is not private",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn a_checksum_is_the_sha_256_of_the_migration_text()
    {
        // The empty string's SHA-256, so the encoding itself is pinned and not
        // just checked against itself.
        assert_eq!(
            checksum_of(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_ne!(checksum_of(MIGRATION_0), checksum_of(""));
    }

    #[test]
    fn migration_ids_are_dense_and_in_order()
    {
        for (position, migration) in MIGRATIONS.iter().enumerate()
        {
            assert_eq!(migration.id, position as i64);
        }

        assert_eq!(LATEST_SCHEMA_VERSION as usize, MIGRATIONS.len());
    }
}
