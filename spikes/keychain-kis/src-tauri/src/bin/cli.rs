//! Scans, from outside the app.
//!
//! It is a separate process because the app's own stdout is one of the artifacts
//! under test, and a scanner running inside the app cannot read the terminal it
//! is writing to. It also has no Keychain access of its own, which keeps the
//! credential path in one binary.

use std::path::PathBuf;
use std::process::ExitCode;

use trdr_spike_keychain_kis_lib::scan::{self, Needle};
use trdr_spike_keychain_kis_lib::scenario::Canaries;
use trdr_spike_keychain_kis_lib::workspace::{self, Workspace};

const USAGE: &str = "\
trdr-spike-kis — the scanning half of the M1 Keychain/KIS spike

    scan [extra-file ...]    search every artifact for the canaries and report
    artifacts                list what would be scanned
";

fn main() -> ExitCode
{
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();

    let root = workspace::product_root();
    let Ok(workspace) = Workspace::open(&root) else
    {
        eprintln!("the spike workspace under {} could not be opened", root.display());
        return ExitCode::FAILURE;
    };

    match words.split_first()
    {
        Some((&"artifacts", _)) =>
        {
            for artifact in targets(&workspace, &[])
            {
                println!("{}", artifact.display());
            }
            ExitCode::SUCCESS
        }
        Some((&"scan", extra)) => report(&workspace, extra),
        _ =>
        {
            eprint!("{USAGE}");
            ExitCode::from(64)
        }
    }
}

fn targets(workspace: &Workspace, extra: &[&str]) -> Vec<PathBuf>
{
    let mut artifacts = workspace.artifacts();
    artifacts.extend(scan::crash_reports("trdr-spike-keychain-kis"));
    artifacts.extend(extra.iter().map(PathBuf::from));
    artifacts
}

fn report(workspace: &Workspace, extra: &[&str]) -> ExitCode
{
    let canaries = Canaries::default();
    let needles = vec![
        Needle::new("app secret", &canaries.app_secret),
        Needle::new("account number", &canaries.account_no),
        Needle::new("access token", "mock-access-token")
    ];

    let artifacts = targets(workspace, extra);
    let report = scan::scan(&artifacts, &needles);

    for scanned in &report.scanned
    {
        println!("scanned  {scanned}");
    }
    for missing in &report.missing
    {
        println!("absent   {missing}");
    }

    // An artifact that was never written is not evidence of anything. Saying so
    // separately keeps a run that did nothing from reading as a clean run.
    if report.scanned.is_empty()
    {
        println!("\nnothing was scanned — run the app first");
        return ExitCode::FAILURE;
    }

    if report.clean()
    {
        println!("\nclean — {} artifacts, no canary", report.scanned.len());
        return ExitCode::SUCCESS;
    }

    println!();
    for hit in &report.hits
    {
        println!("LEAK  {} at byte {} in {}", hit.label, hit.offset, hit.artifact);
    }
    ExitCode::FAILURE
}
