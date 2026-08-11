//! Drives every path the spike has to answer for, once, in one run.
//!
//! It exists as one routine rather than a set of buttons because the pass
//! condition is about artifacts: a canary must be absent from the log, the
//! database, the export and the crash reports *after* the failure paths have
//! run, not after the happy one. A scan that only ever follows a successful call
//! is testing the case the plan says is not where secrets escape.

use serde::Serialize;

use crate::credential::{self, CredentialHandle, CredentialState, CredentialStore};
use crate::http::UreqTransport;
use crate::kis::{AccountSnapshot, Credentials, KisClient, KisError};
use crate::logging::Log;
use crate::mock::{MockKis, Mode};
use crate::secret::Secret;
use crate::store::Store;
use crate::workspace::Workspace;

#[derive(Debug, Clone, Serialize)]
pub struct Step
{
    pub name: String,
    pub code: String
}

#[derive(Debug, Clone, Serialize)]
pub struct Outcome
{
    pub steps: Vec<Step>,
    pub credential: CredentialHandle,
    pub snapshot: Option<AccountSnapshot>
}

/// The two values that must not appear anywhere afterwards.
pub struct Canaries
{
    pub app_secret: String,
    pub account_no: String
}

impl Default for Canaries
{
    fn default() -> Self
    {
        Canaries
        {
            app_secret: "TRDR-CANARY-SECRET-6f2a9c41".to_string(),
            account_no: "TRDR-CANARY-ACCOUNT-50112233".to_string()
        }
    }
}

pub const APP_KEY: &str = "spike-app-key";

struct Run<'a>
{
    log: &'a Log,
    steps: Vec<Step>
}

impl Run<'_>
{
    fn record(&mut self, name: &str, code: &str)
    {
        self.log.event(code, &[("step", name), ("collector", credential::COLLECTOR)]);
        self.steps.push(Step { name: name.to_string(), code: code.to_string() });
    }
}

pub fn run(workspace: &Workspace, instance_id: &str, credentials: &dyn CredentialStore, log: &Log, canaries: &Canaries) -> Outcome
{
    let secret_account = credential::account(instance_id, credential::COLLECTOR, "app_secret");
    let key_account = credential::account(instance_id, credential::COLLECTOR, "app_key");
    let number_account = credential::account(instance_id, credential::COLLECTOR, "account_no");

    let mut run = Run { log, steps: Vec::new() };

    // The Keychain half: store, rotate, and confirm the state moves.
    let stored = credentials.set(&key_account, &Secret::new(APP_KEY)).is_ok()
        && credentials.set(&secret_account, &Secret::new("TRDR-CANARY-SECRET-first")).is_ok()
        && credentials.set(&number_account, &Secret::new(&canaries.account_no)).is_ok();
    run.record("credential.store", if stored { "AUTH_STORED" } else { "AUTH_STORE_REFUSED" });

    let rotated = credentials.set(&secret_account, &Secret::new(&canaries.app_secret)).is_ok();
    run.record("credential.rotate", if rotated { "AUTH_ROTATED" } else { "AUTH_STORE_REFUSED" });
    run.record("credential.state", state_code(credentials.state(&secret_account)));

    let held = Credentials
    {
        app_key: credentials.get(&key_account).ok().flatten().unwrap_or_else(|| Secret::new("")),
        app_secret: credentials.get(&secret_account).ok().flatten().unwrap_or_else(|| Secret::new("")),
        account_no: credentials.get(&number_account).ok().flatten().unwrap_or_else(|| Secret::new(""))
    };

    // Spike-only. Crash artifacts are on the plan's list of places to scan, and
    // the only way to have one is to die while the secrets are in memory. The
    // abort happens here on purpose: `held` is alive, its buffers have not been
    // zeroed by any drop, and this is the worst moment for the process to end.
    if std::env::var("TRDR_SPIKE_CRASH").is_ok()
    {
        run.record("crash", "SPIKE_ABORT");
        std::process::abort();
    }

    let Ok(mock) = MockKis::start(APP_KEY, &canaries.app_secret) else
    {
        run.record("kis.mock", "UPSTREAM_UNAVAILABLE");
        return finish(run, CredentialState::Untested, None, workspace);
    };
    let client = KisClient::new(mock.base_url(), UreqTransport);

    // The happy path, then the refresh the plan warns about.
    let mut snapshot = None;
    let mut state = CredentialState::Untested;
    match client.issue_token(&held).and_then(|token| client.fetch_balance(&held, token))
    {
        Ok((fetched, _)) =>
        {
            run.record("kis.balance", "OK");
            state = CredentialState::Valid;
            snapshot = Some(fetched);
        }
        Err(error) => run.record("kis.balance", error.code())
    }

    mock.set_mode(Mode::TokenExpiredOnce);
    match client.issue_token(&held).and_then(|token| client.fetch_balance(&held, token))
    {
        Ok((fetched, _)) =>
        {
            run.record("kis.refresh", "OK");
            snapshot = Some(fetched);
        }
        Err(error) => run.record("kis.refresh", error.code())
    }

    // Every failure path, because these are the ones that format a request.
    mock.set_mode(Mode::Healthy);
    let wrong = Credentials
    {
        app_key: Secret::new(APP_KEY),
        app_secret: Secret::new("TRDR-CANARY-SECRET-wrong"),
        account_no: Secret::new(&canaries.account_no)
    };
    let refused = client.issue_token(&wrong).err().unwrap_or(KisError::AuthMissing);
    run.record("kis.wrong_secret", refused.code());

    for (mode, name) in [
        (Mode::RateLimited, "kis.rate_limited"),
        (Mode::Unavailable, "kis.unavailable"),
        (Mode::Malformed, "kis.malformed"),
        (Mode::Slow, "kis.timeout")
    ]
    {
        mock.set_mode(mode);
        let error = client.issue_token(&held).err().unwrap_or(KisError::AuthMissing);
        run.record(name, error.code());
        if mode == Mode::RateLimited
        {
            state = CredentialState::RateLimited;
        }
    }

    // And the one that matters for a user who wants out.
    let deleted = credentials.delete(&secret_account).is_ok()
        && credentials.delete(&key_account).is_ok()
        && credentials.delete(&number_account).is_ok();
    run.record("credential.delete", if deleted { "AUTH_DELETED" } else { "AUTH_STORE_REFUSED" });
    run.record("credential.state_after_delete", state_code(credentials.state(&secret_account)));

    finish(run, state, snapshot, workspace)
}

fn finish(run: Run<'_>, state: CredentialState, snapshot: Option<AccountSnapshot>, workspace: &Workspace) -> Outcome
{
    let outcome = Outcome
    {
        steps: run.steps,
        credential: CredentialHandle::new(credential::COLLECTOR, state),
        snapshot
    };

    if let Ok(store) = Store::open(&workspace.db_path())
    {
        if let Some(snapshot) = &outcome.snapshot
        {
            let _ = store.save_snapshot(snapshot);
        }
        let _ = store.save_credential_state(credential::COLLECTOR, state);
        let _ = store.checkpoint();
    }

    // The export of section 6.4: what a user is allowed to take away. It is
    // built from the outcome, so anything the outcome cannot hold cannot get in.
    if let Ok(text) = serde_json::to_string_pretty(&outcome)
    {
        let _ = std::fs::write(workspace.export_path(), text);
    }

    outcome
}

fn state_code(state: CredentialState) -> &'static str
{
    match state
    {
        CredentialState::Missing => "AUTH_MISSING",
        CredentialState::Untested => "AUTH_UNTESTED",
        CredentialState::Valid => "AUTH_VALID",
        CredentialState::Invalid => "AUTH_INVALID",
        CredentialState::RateLimited => "AUTH_RATE_LIMITED"
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::credential::MemoryStore;
    use crate::scan::{scan, Needle};

    fn scratch(name: &str) -> std::path::PathBuf
    {
        let directory = std::env::temp_dir().join(format!("trdr-spike-scenario-{name}"));
        let _ = std::fs::remove_dir_all(&directory);
        directory
    }

    fn drive(name: &str) -> (Workspace, Outcome, Log, Canaries)
    {
        let root = scratch(name);
        let workspace = Workspace::open(&root).expect("the workspace should open");
        let log = Log::new(&workspace.log_path());
        let canaries = Canaries::default();
        let outcome = run(&workspace, "test-instance", &MemoryStore::default(), &log, &canaries);
        (workspace, outcome, log, canaries)
    }

    #[test]
    fn every_path_runs_and_each_reports_its_own_code()
    {
        let (_, outcome, _, _) = drive("all-paths");
        let codes: Vec<(&str, &str)> = outcome
            .steps
            .iter()
            .map(|step| (step.name.as_str(), step.code.as_str()))
            .collect();

        for expected in [
            ("credential.store", "AUTH_STORED"),
            ("credential.rotate", "AUTH_ROTATED"),
            ("credential.state", "AUTH_UNTESTED"),
            ("kis.balance", "OK"),
            ("kis.refresh", "OK"),
            ("kis.wrong_secret", "AUTH_INVALID"),
            ("kis.rate_limited", "AUTH_RATE_LIMITED"),
            ("kis.unavailable", "UPSTREAM_UNAVAILABLE"),
            ("kis.malformed", "UPSTREAM_MALFORMED"),
            ("kis.timeout", "UPSTREAM_TIMEOUT"),
            ("credential.delete", "AUTH_DELETED"),
            ("credential.state_after_delete", "AUTH_MISSING")
        ]
        {
            assert!(codes.contains(&expected), "{expected:?} did not happen; got {codes:?}");
        }
    }

    // The pass condition, run over what a full drive actually wrote.
    #[test]
    fn no_artifact_carries_either_canary_after_every_path_has_run()
    {
        let (workspace, _, _, canaries) = drive("no-canary");
        let needles = vec![
            Needle::new("app secret", &canaries.app_secret),
            Needle::new("account number", &canaries.account_no)
        ];

        let report = scan(&workspace.artifacts(), &needles);
        assert!(report.clean(), "a canary reached an artifact: {:?}", report.hits);
        assert!(report.scanned.iter().any(|path| path.ends_with("app.log")), "the log was not written");
        assert!(report.scanned.iter().any(|path| path.ends_with("today.json")), "the export was not written");
        assert!(report.scanned.iter().any(|path| path.ends_with("trdr.sqlite3")), "the database was not written");
    }

    #[test]
    fn the_log_holds_codes_and_no_free_text()
    {
        let (_, _, log, canaries) = drive("log-shape");
        for line in log.lines()
        {
            assert!(!line.contains(&canaries.app_secret), "the log leaked the secret: {line}");
            assert!(!line.contains(&canaries.account_no), "the log leaked the account number: {line}");
            assert!(line.contains("step="), "a line is not a code with fields: {line}");
        }
    }

    #[test]
    fn the_export_carries_a_state_and_a_snapshot_but_no_credential()
    {
        let (workspace, _, _, canaries) = drive("export-shape");
        let text = std::fs::read_to_string(workspace.export_path()).expect("the export should exist");
        assert!(text.contains("\"state\""), "the export has no credential state");
        assert!(!text.contains(&canaries.app_secret));
        assert!(!text.contains(&canaries.account_no));
        assert!(!text.contains("mock-access-token"));
    }
}
