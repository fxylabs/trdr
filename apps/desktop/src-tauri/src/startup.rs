//! Bringing the runtime up, in the one order that is safe, before any window.
//!
//! `docs/FOUNDATION_DESIGN.md` section 14's stop condition asks for an app that
//! holds the single writer lease and a CLI that finds the same workspace and
//! runtime. That is four steps, and their order is not a convention someone has
//! to remember — each one produces the value the next one asks for:
//!
//! ```text
//! product root       where state lives, from the environment or ~/.trdr
//! → writer lease     this process is now the writer, and can prove it
//! → database         opened under the lease; migration 0 runs on a fresh file
//! → workspace        workspace.json created on a first run (section 5.1)
//! → socket server    bound with a bridge that answers `app.status`
//! ```
//!
//! # Failing closed
//!
//! Every step happens before `tauri::Builder::run`, so a failure at any of them
//! means no window is ever created. The app writes the failure to standard
//! error — a sentence for a person, then the [`ErrorEnvelope`] as one JSON line
//! for anything reading it — and exits non-zero.
//!
//! That is the whole of the second-instance behaviour, and it is deliberately
//! the simplest thing that is honest. A second copy launched while the first is
//! running is refused by the lease at step two, so it never opens the database,
//! never binds over the running app's socket, and never puts up a window that
//! looks like the app but cannot write. macOS will not launch a second copy of a
//! bundled `.app` by double-click at all — it activates the running one — so the
//! case this reaches is a binary started from a terminal or a script, and
//! standard error is where such a caller is already looking.
//!
//! # Nothing slow on this path
//!
//! M1 found that a synchronous six-second read on start-up is indistinguishable
//! from a hung app. Everything here is bounded and local: taking an `flock`,
//! opening SQLite and running one `CREATE TABLE`, writing a 130-byte JSON file,
//! and binding a Unix socket. There is no network, no Keychain prompt, and no
//! directory walk. The socket's accept loop is the only thread this starts, and
//! it is started rather than waited on. Anything later that is not bounded —
//! reading a workspace's objects, testing a collector's credential — belongs
//! behind a command the screen can show progress for, not here.

use crate::commands::{BootstrapState, Queries};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use trdr_core::error::{ErrorCode, ErrorEnvelope, ErrorParam, Retryability};
use trdr_core::socket::{AccountInspectResult, AppStatusResult, DataCoverageResult};
use trdr_runtime::clock::SystemClock;
use trdr_runtime::db::{Database, DbError};
use trdr_runtime::ids::UlidGenerator;
use trdr_runtime::query::{QueryService, SyntheticQueries};
use trdr_runtime::root::{LeaseError, ProductRoot, RootError};
use trdr_runtime::socket::{AppBridge, ServerError, ServerHandle, SocketServer};
use trdr_runtime::workspace::{Workspace, WorkspaceError};

/// The version of the app itself, as the crate manifest states it.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The environment variable that moves the product root somewhere else.
///
/// This is how a test, a second checkout, or a support session runs the app
/// against a directory that is not the one a person's real workspace is in.
/// `trdr-runtime` deliberately has no such fallback — [`ProductRoot`] has to be
/// told where it is — so the whole of "where does this build keep its state"
/// is this one function and the `--product-root` flag the CLI already has.
pub const PRODUCT_ROOT_VARIABLE: &str = "TRDR_PRODUCT_ROOT";

/// Where this process keeps its state.
pub fn product_root() -> Result<ProductRoot, StartupError>
{
    product_root_from(std::env::var_os(PRODUCT_ROOT_VARIABLE))
}

/// The same decision, with the environment handed in so a test can make it.
fn product_root_from(setting: Option<OsString>) -> Result<ProductRoot, StartupError>
{
    match setting.filter(|value| !value.is_empty())
    {
        Some(path) => Ok(ProductRoot::at(PathBuf::from(path))),
        None => Ok(ProductRoot::for_current_user()?)
    }
}

/// The running app's hold on the machine: the lease, the database, the
/// workspace, and the socket.
///
/// Owning this value is what makes the process the app. It is handed to Tauri as
/// managed state so that its lifetime is the window's, and
/// [`AppRuntime::shut_down`] is called on the way out.
pub struct AppRuntime
{
    bootstrap: BootstrapState,
    workspace: Workspace,
    queries: Arc<dyn QueryService>,
    status: Arc<Status>,
    /// `Option` so that shutting down can take the handle out and drop it. The
    /// handle's own `Drop` stops the accept loop, joins it, and unlinks the
    /// socket file, so what shutting down needs is to make that happen at a
    /// moment of its choosing rather than to reimplement it.
    server: Mutex<Option<ServerHandle>>
}

impl AppRuntime
{
    /// Runs the four steps, or says which one failed.
    ///
    /// A failure anywhere releases everything taken so far: the lease lives in
    /// an `Arc` that is dropped when this function returns, and dropping it
    /// unlocks the `flock`. So a refused start leaves no lease behind for the
    /// next attempt to trip over.
    pub fn start(root: ProductRoot) -> Result<Self, StartupError>
    {
        let lease = Arc::new(root.acquire_writer_lease()?);
        let database = Database::open(Arc::clone(&lease))?;
        let workspace = Workspace::open_default(&lease, &SystemClock, &UlidGenerator)?;
        let queries: Arc<dyn QueryService> = Arc::new(SyntheticQueries::load(SystemClock)?);

        let schema_version = database.schema_version()?;
        let bootstrap = BootstrapState {
            workspace_id: workspace.id(),
            workspace_path: workspace.path().to_path_buf(),
            schema_version
        };

        let status = Arc::new(Status {
            product_root: root.path().to_path_buf(),
            workspace_path: workspace.path().to_path_buf(),
            database: Mutex::new(database),
            schema_version_at_open: schema_version,
            queries: Arc::clone(&queries)
        });

        let server = SocketServer::bind(lease, Arc::clone(&status) as Arc<dyn AppBridge>)?.spawn();

        Ok(Self {
            bootstrap,
            workspace,
            queries,
            status,
            server: Mutex::new(Some(server))
        })
    }

    /// What `bootstrap.get` answers from.
    pub fn bootstrap(&self) -> &BootstrapState
    {
        &self.bootstrap
    }

    /// The workspace this app has open.
    pub fn workspace(&self) -> &Workspace
    {
        &self.workspace
    }

    /// The query service, for Tauri to manage.
    ///
    /// A clone of the same `Arc` the socket bridge holds, which is the whole
    /// point: milestone M2 requires the UI and the CLI to read one domain
    /// object, and sharing the service rather than building two is what makes
    /// that impossible to get wrong later.
    pub fn queries(&self) -> Queries
    {
        Queries(Arc::clone(&self.queries))
    }

    /// The same answer the CLI gets over the socket.
    pub fn status(&self) -> AppStatusResult
    {
        self.status.app_status()
    }

    /// Stops serving and removes the socket file.
    ///
    /// Called from `RunEvent::Exit` rather than left to `Drop`, because the
    /// event loop underneath Tauri ends the process itself on some paths, and a
    /// destructor that never runs is a socket file that outlives its app.
    ///
    /// It does not release the writer lease, and deliberately so. The lease is
    /// held by an open descriptor, and the kernel drops it when the process
    /// ends — which is the same thing that happens to a process that is killed.
    /// Leaving it to that means the lease has one release path rather than two,
    /// and the one it has is the one that also covers the crash. The socket file
    /// is the only thing that needs saying out loud, because nothing collects it
    /// automatically.
    ///
    /// Safe to call more than once; the second call finds nothing to stop.
    pub fn shut_down(&self)
    {
        if let Ok(mut server) = self.server.lock()
        {
            drop(server.take());
        }
    }
}

/// The [`AppBridge`] the socket server answers `app.status` through.
struct Status
{
    product_root: PathBuf,
    workspace_path: PathBuf,
    /// Held open for the life of the app. It carries the writer lease, so this
    /// is also what stops the lease from being released while the app is up.
    database: Mutex<Database>,
    /// What the database said its schema version was when it was opened.
    schema_version_at_open: i32,
    /// The same service the WebView's commands answer from.
    queries: Arc<dyn QueryService>
}

impl Status
{
    /// The schema version, read back out of the open database.
    ///
    /// `app.status` has one result shape and no error arm (section 9.2), so a
    /// connection that cannot answer falls back to what it said when it was
    /// opened. The two can only differ if the connection has since failed, and
    /// this build changes the schema exactly once — during the open — so the
    /// fallback is not a stale guess but the same number.
    fn schema_version(&self) -> i64
    {
        self.database
            .lock()
            .ok()
            .and_then(|database| database.schema_version().ok())
            .map_or(i64::from(self.schema_version_at_open), i64::from)
    }
}

impl AppBridge for Status
{
    /// `account.inspect`, answered from the Today model.
    ///
    /// The CLI and the screen therefore cannot disagree about what the account
    /// holds: there is one model, built once, and this takes two of its fields.
    /// Section 9.2 requires a bounded read with no secret and no raw account
    /// payload, and the model has neither to give.
    fn account_inspect(&self) -> Result<AccountInspectResult, ErrorEnvelope>
    {
        let today = self.queries.today()?;

        Ok(AccountInspectResult {
            origin: today.header.origin,
            account: today.account,
            holdings: today.holdings
        })
    }

    /// `data.coverage`, answered from the Lab draft model.
    fn data_coverage(&self) -> Result<DataCoverageResult, ErrorEnvelope>
    {
        let draft = self.queries.lab_draft()?;

        Ok(DataCoverageResult {
            origin: draft.header.origin,
            coverage: draft.coverage
        })
    }

    fn app_status(&self) -> AppStatusResult
    {
        AppStatusResult {
            app_version: APP_VERSION.to_owned(),
            pid: std::process::id(),
            // Not read from anywhere: `AppRuntime::start` could not have built
            // this value without the lease, and the database below is still
            // holding it.
            holds_writer_lease: true,
            product_root: self.product_root.clone(),
            workspace_path: self.workspace_path.clone(),
            schema_version: self.schema_version()
        }
    }
}

/// The app could not be brought up.
#[derive(Debug, thiserror::Error)]
pub enum StartupError
{
    /// The compiled-in synthetic fixture could not be read.
    #[error("the synthetic fixture could not be read")]
    Fixture(ErrorEnvelope),
    /// The product root could not be located or prepared.
    #[error(transparent)]
    Root(#[from] RootError),
    /// The writer lease could not be taken, most often because another copy of
    /// the app holds it.
    #[error(transparent)]
    Lease(#[from] LeaseError),
    /// The database could not be opened or migrated.
    #[error(transparent)]
    Database(#[from] DbError),
    /// The workspace could not be created or read.
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    /// The socket could not be bound.
    #[error(transparent)]
    Server(#[from] ServerError)
}

impl From<ErrorEnvelope> for StartupError
{
    fn from(envelope: ErrorEnvelope) -> Self
    {
        Self::Fixture(envelope)
    }
}

impl StartupError
{
    /// Whether another live process already holds the writer lease.
    pub fn is_lease_held(&self) -> bool
    {
        matches!(self, Self::Lease(LeaseError::Held { .. }))
    }

    /// The same failure in the one shape every surface renders (section 12).
    ///
    /// Section 12's code set is closed, and it has no start-up family, so every
    /// failure here is placed in an existing one and says which start-up step it
    /// was in a `reason` parameter. The placements, and why:
    ///
    /// - The lease being held is `DB_BUSY` — literally "the database is held by
    ///   someone else right now", which is what a second copy has run into.
    /// - A workspace or database written by a later build is `DATA_UNSUPPORTED`:
    ///   the data is of a kind this version does not support, and section 6.1
    ///   forbids opening it rather than guessing.
    /// - Damage, drift, and a missing recovery point stay in the `DB_` family,
    ///   which is where the design already puts them.
    /// - Everything else is the environment refusing to give the app a place to
    ///   run, and reports as `APP_NOT_RUNNING` — which is also what `trdr app
    ///   status` will say a moment later, and for the same reason. The CLI
    ///   already uses that code with `reason=home_unknown` for the identical
    ///   case.
    ///
    /// No sentence crosses this boundary; the wording is [`report`]'s.
    pub fn to_envelope(&self) -> ErrorEnvelope
    {
        if let Self::Fixture(envelope) = self
        {
            return envelope.clone().with_param(
                "reason",
                ErrorParam::literal("synthetic_fixture_unreadable")
            );
        }

        let (code, reason) = match self
        {
            Self::Root(RootError::HomeUnknown) => (ErrorCode::AppNotRunning, "home_unknown"),
            Self::Root(RootError::Prepare { .. }) =>
            {
                (ErrorCode::AppNotRunning, "product_root_unusable")
            }
            Self::Lease(LeaseError::Held { .. }) => (ErrorCode::DbBusy, "writer_lease_held"),
            Self::Lease(LeaseError::Open { .. } | LeaseError::Root(_)) =>
            {
                (ErrorCode::AppNotRunning, "writer_lease_unusable")
            }
            Self::Database(error) => database_reason(error),
            Self::Workspace(WorkspaceError::Manifest { source, .. }) => match source
            {
                trdr_core::workspace::ManifestError::UnsupportedSchemaVersion { .. } =>
                {
                    (ErrorCode::DataUnsupported, "workspace_schema_unsupported")
                }
                trdr_core::workspace::ManifestError::Unreadable =>
                {
                    (ErrorCode::DbIntegrity, "workspace_manifest_unreadable")
                }
            },
            Self::Workspace(_) => (ErrorCode::AppNotRunning, "workspace_unusable"),
            Self::Server(_) => (ErrorCode::AppNotRunning, "socket_unavailable"),
            // Answered above, before the codes that have no envelope of their own.
            Self::Fixture(_) => (ErrorCode::DataIntegrity, "synthetic_fixture_unreadable")
        };

        ErrorEnvelope::new(code)
            .with_param("reason", ErrorParam::literal(reason))
            .with_retryability(Retryability::No)
    }
}

/// Where in section 12 a database failure belongs, and what it was.
fn database_reason(error: &DbError) -> (ErrorCode, &'static str)
{
    match error
    {
        DbError::SchemaFromNewerBuild { .. } =>
        {
            (ErrorCode::DataUnsupported, "schema_from_newer_build")
        }
        DbError::MigrationDrift { .. } => (ErrorCode::DbMigration, "migration_drift"),
        DbError::UnknownAppliedMigration { .. } =>
        {
            (ErrorCode::DbMigration, "migration_from_a_newer_build")
        }
        DbError::RecoveryPointRequired { .. } =>
        {
            (ErrorCode::DbMigration, "recovery_point_required")
        }
        DbError::ForeignKeyViolations { .. } => (ErrorCode::DbMigration, "foreign_key_violations"),
        DbError::IntegrityCheckFailed => (ErrorCode::DbIntegrity, "integrity_check_failed"),
        DbError::NotATrdrDatabase { .. } => (ErrorCode::DbIntegrity, "not_a_trdr_database"),
        DbError::SqliteTooOld { .. } => (ErrorCode::DbIntegrity, "bundled_sqlite_too_old"),
        DbError::WalRefused { .. } => (ErrorCode::DbIntegrity, "write_ahead_logging_refused"),
        DbError::ForeignKeysRefused => (ErrorCode::DbIntegrity, "foreign_keys_refused"),
        DbError::Permissions { .. } => (ErrorCode::AppNotRunning, "state_files_unwritable"),
        DbError::Root(_) => (ErrorCode::AppNotRunning, "product_root_unusable"),
        DbError::Sqlite(_) => (ErrorCode::DbIntegrity, "database_unopenable")
    }
}

/// Writes the failure where a caller will see it, and ends the process.
///
/// Two lines, on standard error, in this order: a sentence a person can act on,
/// and the envelope as JSON for anything parsing the output. The sentence lives
/// here rather than in `trdr-runtime` because section 3.1 puts user-facing
/// wording in the surface, and this crate is a surface.
pub fn report_and_exit(error: &StartupError) -> !
{
    let envelope = error.to_envelope();

    eprintln!("trdr could not start: {}", sentence(error));

    if let Ok(json) = serde_json::to_string(&envelope)
    {
        eprintln!("{json}");
    }

    std::process::exit(1)
}

/// What to tell the person who started the app.
fn sentence(error: &StartupError) -> &'static str
{
    match error
    {
        _ if error.is_lease_held() =>
        {
            "another copy of trdr is already running and holds the writer lease. \
             Quit it and try again."
        }
        StartupError::Root(RootError::HomeUnknown) =>
        {
            "the home directory could not be found, so there is nowhere to keep \
             its state."
        }
        StartupError::Root(_) | StartupError::Lease(_) =>
        {
            "its state directory could not be prepared."
        }
        StartupError::Database(_) =>
        {
            "the workspace database could not be opened. Run `trdr app status` \
             for the code, and do not delete anything in the workspace."
        }
        StartupError::Workspace(_) => "the workspace could not be read.",
        StartupError::Fixture(_) =>
        {
            "the synthetic data this build ships with could not be read. This is a \
             fault in the build itself rather than in the workspace."
        }
        StartupError::Server(_) => "its command socket could not be opened."
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use trdr_runtime::test_support::scratch_root;

    #[test]
    fn the_product_root_comes_from_the_environment_when_it_says_so()
    {
        let chosen = product_root_from(Some(OsString::from("/private/tmp/trdr-t/chosen")))
            .expect("an explicit root is always usable");

        assert_eq!(
            chosen.path(),
            std::path::Path::new("/private/tmp/trdr-t/chosen")
        );
    }

    /// An empty variable is a variable someone exported and did not fill in.
    /// Treating it as "the root is the empty path" would put the whole product
    /// in the working directory.
    #[test]
    fn an_empty_setting_falls_back_rather_than_naming_the_current_directory()
    {
        let fallback = product_root_from(Some(OsString::new())).expect("a home is set here");

        assert!(fallback.path().ends_with(".trdr"), "{fallback:?}");
        assert!(fallback.path().is_absolute());
    }

    /// Nothing in this test touches the filesystem: a [`ProductRoot`] is a path
    /// and creating one creates no directory. The whole of this crate's test
    /// suite reaches `~/.trdr` at no other point, which is checked in
    /// `tests/startup.rs`.
    #[test]
    fn without_a_setting_the_root_is_the_one_section_fifteen_fixes()
    {
        let default = product_root_from(None).expect("a home is set here");

        assert_eq!(default.path().file_name().unwrap(), ".trdr");
    }

    #[test]
    fn a_second_start_against_a_held_root_is_refused_as_a_busy_database()
    {
        let root = scratch_root("startup-second");
        let first = AppRuntime::start(root.clone()).expect("the first start should work");

        let Err(refused) = AppRuntime::start(root)
        else
        {
            panic!("a second start took a lease the first one is holding");
        };

        assert!(refused.is_lease_held());
        assert_eq!(refused.to_envelope().code, ErrorCode::DbBusy);
        assert_eq!(
            refused.to_envelope().params.get("reason"),
            Some(&ErrorParam::literal("writer_lease_held"))
        );

        first.shut_down();
    }

    /// Every failure that can come out of start-up renders as a code section 12
    /// defines, carrying a reason. The point is not any single mapping but that
    /// none of them falls through to something unnamed.
    #[test]
    fn every_startup_failure_renders_as_a_code_and_a_reason()
    {
        let failures = [
            StartupError::Root(RootError::HomeUnknown),
            StartupError::Lease(LeaseError::Held {
                path: PathBuf::from("/private/tmp/t/run/writer.lock")
            }),
            StartupError::Database(DbError::IntegrityCheckFailed),
            StartupError::Database(DbError::MigrationDrift { id: 0 }),
            StartupError::Database(DbError::SchemaFromNewerBuild {
                found: 9,
                supported: 1
            }),
            StartupError::Workspace(WorkspaceError::Manifest {
                path: PathBuf::from("/private/tmp/t/workspaces/default/workspace.json"),
                source: trdr_core::workspace::ManifestError::Unreadable
            }),
            StartupError::Server(ServerError::PathTooLong {
                found: 200,
                limit: 103
            })
        ];

        for failure in &failures
        {
            let envelope = failure.to_envelope();

            assert!(
                envelope.params.contains_key("reason"),
                "{failure:?} carries no reason"
            );
            assert!(
                trdr_core::ErrorCode::ALL.contains(&envelope.code),
                "{failure:?} rendered as a code section 12 does not define"
            );
            assert!(!sentence(failure).is_empty());
        }
    }
}
