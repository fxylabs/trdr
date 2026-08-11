//! The writer lease, proven between real processes.
//!
//! `flock` is held by an open file description rather than by a process, so a
//! second `open` plus `flock` inside the same process is allowed to succeed on
//! some platforms and proves nothing. Threads do not help either — they share the
//! process. The only honest proof spawns another process, so these tests
//! re-execute this test binary with an environment variable telling the child
//! what to do, and read the answer from its exit status.
//!
//! Every test runs against a scratch product root under `/private/tmp`. Nothing
//! here can reach the `~/.trdr` a person is using.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use trdr_runtime::db::Database;
use trdr_runtime::root::{LeaseError, ProductRoot};
use trdr_runtime::test_support::scratch_root;

/// Where the child should try to take the lease.
const ROOT_VARIABLE: &str = "TRDR_TEST_LEASE_ROOT";

/// Set when the child should keep the lease rather than report and exit.
const HOLD_VARIABLE: &str = "TRDR_TEST_LEASE_HOLD";

/// The child took the lease.
const EXIT_TOOK: i32 = 10;

/// The child was refused because someone else holds it.
const EXIT_REFUSED: i32 = 11;

/// The child failed for some other reason.
const EXIT_FAILED: i32 = 12;

/// What a holding child prints once it has the lease.
const HELD_LINE: &str = "held";

/// The child half of every test in this file.
///
/// Ordinarily this is a test that does nothing. Re-executed with
/// `TRDR_TEST_LEASE_ROOT` set it becomes the second process, and its exit status
/// is the answer the parent reads.
#[test]
fn lease_child()
{
    let Some(root) = std::env::var_os(ROOT_VARIABLE)
    else
    {
        return;
    };

    let holding = std::env::var_os(HOLD_VARIABLE).is_some();
    let root = ProductRoot::at(PathBuf::from(root));

    match root.acquire_writer_lease()
    {
        Ok(lease) =>
        {
            if !holding
            {
                std::process::exit(EXIT_TOOK);
            }

            // Say so, then wait to be killed. Being killed is the point: it is
            // how the test reaches the case where a holder dies without ever
            // releasing anything.
            println!("{HELD_LINE}");
            std::thread::sleep(Duration::from_secs(120));
            drop(lease);
            std::process::exit(EXIT_TOOK);
        }
        Err(LeaseError::Held { .. }) => std::process::exit(EXIT_REFUSED),
        Err(_) => std::process::exit(EXIT_FAILED)
    }
}

#[test]
fn a_second_process_is_refused_while_the_first_holds_the_lease()
{
    let root = scratch_root("lease-two-processes");
    let held = root
        .acquire_writer_lease()
        .expect("the lease should be free");

    assert_eq!(
        run_child(&root),
        EXIT_REFUSED,
        "a second process took a lease that was already held"
    );

    drop(held);

    assert_eq!(
        run_child(&root),
        EXIT_TOOK,
        "a released lease should be takeable by another process"
    );
}

// The crash case, for real. A holder that is killed never runs a destructor and
// never writes anything, and the next start still gets the lease, because the
// kernel released it when the descriptor closed. A pid written into a file could
// not have produced this answer.
#[test]
fn a_lease_held_by_a_process_that_is_killed_is_released_by_the_kernel()
{
    let root = scratch_root("lease-killed");
    let mut holder = spawn_holder(&root);

    assert_eq!(
        run_child(&root),
        EXIT_REFUSED,
        "the holder is alive, so nothing else should get the lease"
    );

    holder.kill().expect("the holder could not be killed");
    holder.wait().expect("the holder could not be reaped");

    // The lock file is still there, and still says nothing about who holds it.
    assert!(root.lease_path().exists());

    assert_eq!(
        run_child(&root),
        EXIT_TOOK,
        "a lease left by a killed process blocked the next one"
    );
}

// The order the brief asks for, from the other side: the database is opened
// through a lease, so a process that cannot take the lease cannot open the
// database either. There is no second door.
#[test]
fn a_process_without_the_lease_cannot_open_the_database()
{
    let root = scratch_root("lease-before-db");
    let first = Arc::new(
        root.acquire_writer_lease()
            .expect("the lease should be free")
    );
    let database = Database::open(Arc::clone(&first)).expect("the database should open");

    assert_eq!(database.schema_version().unwrap(), 1);
    assert_eq!(
        run_child(&root),
        EXIT_REFUSED,
        "a second process could have taken the lease and opened the database"
    );
}

#[test]
fn the_database_opens_again_once_the_lease_moves_on()
{
    let root = scratch_root("lease-handover");
    let lease = Arc::new(
        root.acquire_writer_lease()
            .expect("the lease should be free")
    );
    let database = Database::open(Arc::clone(&lease)).expect("the database should open");
    let applied = database.applied_migrations().unwrap();

    drop(database);
    drop(lease);

    let again = Arc::new(
        root.acquire_writer_lease()
            .expect("the lease should be free")
    );
    let database = Database::open(again).expect("the database should open again");

    assert_eq!(
        database.applied_migrations().unwrap(),
        applied,
        "re-opening applied a migration that had already run"
    );
}

/// Runs the child once and gives back its exit status.
fn run_child(root: &ProductRoot) -> i32
{
    let output = child_command(root)
        .output()
        .expect("the child process could not be run");

    output.status.code().unwrap_or_else(|| {
        panic!(
            "the child was killed by a signal: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// Starts a child that takes the lease and keeps it, returning once it has.
fn spawn_holder(root: &ProductRoot) -> Child
{
    let mut child = child_command(root)
        .env(HOLD_VARIABLE, "1")
        .stdout(Stdio::piped())
        .spawn()
        .expect("the holding child could not be started");

    let stdout = child
        .stdout
        .take()
        .expect("the child has no standard output");
    let deadline = Instant::now() + Duration::from_secs(30);

    for line in BufReader::new(stdout).lines()
    {
        match line
        {
            Ok(line) if line.contains(HELD_LINE) => return child,
            Ok(_) =>
            {}
            Err(error) => panic!("the child's output could not be read: {error}")
        }

        assert!(Instant::now() < deadline, "the child never took the lease");
    }

    panic!("the child ended without taking the lease");
}

/// This test binary, told to run the child test and nothing else.
fn child_command(root: &ProductRoot) -> Command
{
    let mut command =
        Command::new(std::env::current_exe().expect("this test binary could not be located"));

    command
        .args(["lease_child", "--exact", "--nocapture", "--test-threads=1"])
        .env(ROOT_VARIABLE, root.path())
        .env_remove(HOLD_VARIABLE);

    command
}
