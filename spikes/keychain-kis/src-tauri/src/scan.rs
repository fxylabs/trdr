use std::path::{Path, PathBuf};

use serde::Serialize;

/// One place a canary was found. Any hit at all fails the spike.
#[derive(Debug, Clone, Serialize)]
pub struct Hit
{
    pub artifact: String,
    pub offset: usize,
    pub label: String
}

#[derive(Debug, Clone, Serialize)]
pub struct Report
{
    pub scanned: Vec<String>,
    pub missing: Vec<String>,
    pub hits: Vec<Hit>
}

impl Report
{
    pub fn clean(&self) -> bool
    {
        self.hits.is_empty()
    }
}

/// What to look for. The label is what a report prints, so it can name the thing
/// without being the thing.
pub struct Needle
{
    pub label: String,
    pub bytes: Vec<u8>
}

impl Needle
{
    pub fn new(label: &str, value: &str) -> Needle
    {
        Needle { label: label.to_string(), bytes: value.as_bytes().to_vec() }
    }
}

/// Reads bytes, not text.
///
/// The database, its write-ahead log and a crash report are not UTF-8, and a
/// scanner that reads them as a string decides what a byte means before
/// searching it. Anything that reordered or replaced a byte on the way in could
/// hide a match; comparing raw bytes cannot.
pub fn scan(artifacts: &[PathBuf], needles: &[Needle]) -> Report
{
    let mut report = Report { scanned: Vec::new(), missing: Vec::new(), hits: Vec::new() };

    for artifact in artifacts
    {
        let Ok(bytes) = std::fs::read(artifact) else
        {
            report.missing.push(artifact.display().to_string());
            continue;
        };
        report.scanned.push(artifact.display().to_string());

        for needle in needles
        {
            for offset in occurrences(&bytes, &needle.bytes)
            {
                report.hits.push(Hit
                {
                    artifact: artifact.display().to_string(),
                    offset,
                    label: needle.label.clone()
                });
            }
        }
    }

    report
}

fn occurrences(haystack: &[u8], needle: &[u8]) -> Vec<usize>
{
    if needle.is_empty() || haystack.len() < needle.len()
    {
        return Vec::new();
    }
    haystack
        .windows(needle.len())
        .enumerate()
        .filter(|(_, window)| *window == needle)
        .map(|(offset, _)| offset)
        .collect()
}

/// macOS crash reports for one process name.
///
/// A crash is one of the artifacts the plan names, and it is the one nobody
/// writes on purpose — which is why it has to be looked for where the system
/// puts it rather than where the app writes its own files.
pub fn crash_reports(process_name: &str) -> Vec<PathBuf>
{
    let home = std::env::var("HOME").unwrap_or_default();
    let directory = Path::new(&home).join("Library/Logs/DiagnosticReports");
    let Ok(entries) = std::fs::read_dir(directory) else { return Vec::new() };

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.file_name().is_some_and(|name| name.to_string_lossy().starts_with(process_name)))
        .collect()
}

#[cfg(test)]
mod tests
{
    use super::*;

    const CANARY: &str = "TRDR-CANARY-6f2a9c";

    fn scratch(name: &str) -> PathBuf
    {
        let directory = std::env::temp_dir().join(format!("trdr-spike-scan-{name}"));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the scratch directory could not be made");
        directory
    }

    fn needles() -> Vec<Needle>
    {
        vec![Needle::new("app secret", CANARY)]
    }

    #[test]
    fn a_clean_artifact_produces_no_hit()
    {
        let directory = scratch("clean");
        let file = directory.join("app.log");
        std::fs::write(&file, "AUTH_INVALID collector=kis-spike\n").expect("the file should write");

        let report = scan(&[file], &needles());
        assert!(report.clean());
        assert_eq!(report.scanned.len(), 1);
    }

    // The scanner has to be able to fail, or a clean run says nothing. This is
    // the negative control.
    #[test]
    fn a_planted_canary_is_found_with_its_position()
    {
        let directory = scratch("planted");
        let file = directory.join("app.log");
        std::fs::write(&file, format!("AUTH_INVALID appsecret={CANARY}\n")).expect("the file should write");

        let report = scan(&[file], &needles());
        assert!(!report.clean());
        assert_eq!(report.hits.len(), 1);
        assert_eq!(report.hits[0].label, "app secret");
        assert_eq!(report.hits[0].offset, 23);
    }

    #[test]
    fn a_canary_inside_a_binary_file_is_still_found()
    {
        let directory = scratch("binary");
        let file = directory.join("trdr.sqlite3");
        let mut bytes = vec![0u8, 1, 2, 255, 254];
        bytes.extend_from_slice(CANARY.as_bytes());
        bytes.extend_from_slice(&[0u8, 0, 7]);
        std::fs::write(&file, &bytes).expect("the file should write");

        let report = scan(&[file], &needles());
        assert_eq!(report.hits.len(), 1);
        assert_eq!(report.hits[0].offset, 5);
    }

    // An artifact that was never written is not a pass. A scan that silently
    // treats "no file" as "clean" reports success for a run that did nothing.
    #[test]
    fn an_artifact_that_does_not_exist_is_reported_as_missing_not_as_clean()
    {
        let report = scan(&[scratch("absent").join("never-written.log")], &needles());
        assert!(report.clean());
        assert_eq!(report.missing.len(), 1);
        assert!(report.scanned.is_empty());
    }

    #[test]
    fn every_occurrence_is_reported_not_only_the_first()
    {
        let directory = scratch("twice");
        let file = directory.join("today.json");
        std::fs::write(&file, format!("{CANARY} and again {CANARY}")).expect("the file should write");

        assert_eq!(scan(&[file], &needles()).hits.len(), 2);
    }
}
