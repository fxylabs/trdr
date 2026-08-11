use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A log that takes a code and named fields, and has no way to take a sentence.
///
/// Section 12: an error exposes a code, safe message parameters, retryability
/// and a cause chain id — and an HTTP header or body, a credential or an account
/// number is never a log field. A sink with a `message: &str` parameter invites
/// the opposite, because the quickest way to explain a failure is to paste what
/// failed. There is no such parameter here.
pub struct Log
{
    path: PathBuf,
    lines: Mutex<Vec<String>>
}

impl Log
{
    pub fn new(path: &Path) -> Log
    {
        Log { path: path.to_path_buf(), lines: Mutex::new(Vec::new()) }
    }

    pub fn event(&self, code: &str, fields: &[(&str, &str)])
    {
        let rendered: Vec<String> = fields.iter().map(|(name, value)| format!("{name}={value}")).collect();
        let line = if rendered.is_empty() { code.to_string() } else { format!("{code} {}", rendered.join(" ")) };

        self.lines.lock().unwrap().push(line.clone());
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&self.path)
        {
            let _ = writeln!(file, "{line}");
        }
    }

    /// What was written, for a test that wants to assert on it without reading
    /// the file back.
    pub fn lines(&self) -> Vec<String>
    {
        self.lines.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn an_event_writes_its_code_and_fields()
    {
        let path = std::env::temp_dir().join("trdr-spike-log-event.log");
        let _ = std::fs::remove_file(&path);

        let log = Log::new(&path);
        log.event("AUTH_INVALID", &[("collector", "kis-spike"), ("retryable", "false")]);
        log.event("UPSTREAM_TIMEOUT", &[]);

        assert_eq!(log.lines(), vec![
            "AUTH_INVALID collector=kis-spike retryable=false".to_string(),
            "UPSTREAM_TIMEOUT".to_string()
        ]);
        let written = std::fs::read_to_string(&path).expect("the log file should read");
        assert!(written.contains("AUTH_INVALID collector=kis-spike"));
    }

    /// The mechanical half of the redaction rule.
    ///
    /// [`crate::secret::Secret`] makes a value impossible to print by accident,
    /// so the only way one reaches a log is a deliberate call to its single
    /// accessor. That makes the call sites countable, and this asserts which
    /// files are allowed to have any. A new one in the logging, storage,
    /// scanning or Tauri layers fails here rather than at a scan of an artifact
    /// that was already written.
    ///
    /// The accessor is never named literally in this file, so that the check
    /// does not match its own source.
    #[test]
    fn only_the_modules_that_must_touch_a_secret_call_expose()
    {
        let allowed = ["credential.rs", "kis.rs", "secret.rs", "sheet.rs"];
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        // Assembled rather than written out, so that this file does not match
        // its own search and report itself.
        let needle = format!(".{}()", "expose");

        let mut offenders = Vec::new();
        let mut visit = vec![source];
        while let Some(directory) = visit.pop()
        {
            for entry in std::fs::read_dir(&directory).expect("the source directory should read").flatten()
            {
                let path = entry.path();
                if path.is_dir()
                {
                    visit.push(path);
                    continue;
                }
                if path.extension().is_none_or(|extension| extension != "rs")
                {
                    continue;
                }
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                if text.contains(&needle) && !allowed.contains(&name.as_str())
                {
                    offenders.push(name);
                }
            }
        }

        assert!(offenders.is_empty(), "these files reach for a secret's value and should not: {offenders:?}");
    }
}
