use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

// The pitfall the plan names: a GUI app launched from Finder or `open` inherits
// launchd's PATH, not the user's shell PATH, so `claude` and `codex` are
// invisible to it however plainly they work in a terminal. Everything here
// exists to make that difference visible and then to work around it.
#[derive(Serialize)]
pub struct EnvReport
{
    pub login_shell: String,
    pub app_path: String,
    pub login_path: Option<String>
}

pub fn report() -> EnvReport
{
    EnvReport
    {
        login_shell: login_shell(),
        app_path: std::env::var("PATH").unwrap_or_default(),
        login_path: login_path()
    }
}

fn login_shell() -> String
{
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
}

// `-l` is what makes this worth doing: it reads the login profile, which is
// where a user's PATH additions actually live.
pub fn login_path() -> Option<String>
{
    let output = Command::new(login_shell()).arg("-lc").arg("printf %s \"$PATH\"").output().ok()?;
    let answer = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() && !answer.is_empty() { Some(answer) } else { None }
}

// The login shell is asked for its PATH and nothing else; the lookup itself
// happens here. Asking it to resolve the name instead — `command -v` — answers
// with builtins, functions and aliases, which are not things a PTY can spawn:
// `command -v echo` is the word `echo`, not `/bin/echo`. Keeping the name out
// of the shell also means user input never reaches a shell as source.
pub fn resolve_program(name: &str) -> Option<String>
{
    if name.is_empty()
    {
        return None;
    }
    if name.contains('/')
    {
        let path = std::fs::canonicalize(name).ok()?;
        return executable(&path).then(|| path.display().to_string());
    }
    login_path()?
        .split(':')
        .filter(|directory| !directory.is_empty())
        .map(|directory| PathBuf::from(directory).join(name))
        .find(|candidate| executable(candidate))
        .map(|candidate| candidate.display().to_string())
}

fn executable(path: &Path) -> bool
{
    let Ok(meta) = std::fs::metadata(path) else { return false };
    meta.is_file() && meta.permissions().mode() & 0o111 != 0
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn an_absolute_path_is_taken_as_given()
    {
        assert_eq!(resolve_program("/bin/echo").as_deref(), Some("/bin/echo"));
    }

    // The regression this module was rewritten for: the login shell answers
    // `echo` for `echo`, and a shell builtin is not a spawnable executable.
    #[test]
    fn a_bare_name_resolves_to_an_executable_file_not_a_shell_builtin()
    {
        let found = resolve_program("echo").expect("no `echo` executable was found on the login PATH");
        assert!(found.starts_with('/'), "expected an absolute path, got {found}");
        assert!(executable(Path::new(&found)));
    }

    #[test]
    fn a_name_that_does_not_exist_resolves_to_nothing()
    {
        assert!(resolve_program("trdr-no-such-executable-9c1f").is_none());
        assert!(resolve_program("").is_none());
    }

    // The name never reaches a shell as source. If it did, this would run `id`.
    #[test]
    fn a_name_carrying_shell_syntax_cannot_run_a_second_command()
    {
        assert!(resolve_program("nope\"; id; echo \"").is_none());
    }

    // A directory on the PATH shares its name with nothing spawnable, and a
    // non-executable file is not a program however well its name matches.
    #[test]
    fn a_directory_or_a_plain_file_is_not_accepted_as_a_program()
    {
        assert!(!executable(Path::new("/bin")));
        assert!(!executable(Path::new("/etc/hosts")));
    }

    #[test]
    fn the_report_states_both_paths_without_inventing_one()
    {
        let report = report();
        assert!(report.login_shell.starts_with('/'));
        assert!(!report.app_path.is_empty());
    }
}
