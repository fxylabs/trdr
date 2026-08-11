//! Finding the agent executable the way the person who installed it would.
//!
//! The pitfall this module exists for, found in the M1 spike: a GUI app started
//! from Finder or `open` inherits launchd's `PATH`, not the login shell's. A
//! `claude` or `codex` that works perfectly in a terminal is simply invisible to
//! the app, and the failure looks like a missing install rather than a missing
//! environment. So the login shell is asked what its `PATH` is, once, and the
//! lookup happens here.
//!
//! # Why the shell is asked for `PATH` and not for the answer
//!
//! `command -v claude` through a shell is the obvious shortcut and it is wrong
//! twice over. It answers with builtins, functions, and aliases, which are not
//! things a pseudo-terminal can spawn — `command -v echo` is the word `echo`,
//! not `/bin/echo` — and it puts a name that came from outside into a shell as
//! source. Keeping the name out of the shell means no input this module is given
//! can ever run a second command, which [`tests`] holds directly.
//!
//! The shell still runs, with a fixed script that prints one variable. That is
//! the whole of the trust placed in it.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The shell to ask, from the environment, or the macOS default.
///
/// `SHELL` is what the user's login shell is recorded as, and a machine where it
/// is unset is a machine where zsh is the shell macOS gave the account.
pub fn login_shell() -> String
{
    std::env::var("SHELL")
        .ok()
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| "/bin/zsh".to_owned())
}

/// The `PATH` the login shell has, or nothing if it could not be asked.
///
/// `-l` is what makes this worth doing at all: it reads the login profile, which
/// is where a person's `PATH` additions actually live.
pub fn login_path() -> Option<String>
{
    let output = Command::new(login_shell())
        .arg("-lc")
        .arg("printf %s \"$PATH\"")
        .output()
        .ok()?;

    let answer = String::from_utf8_lossy(&output.stdout).trim().to_owned();

    match output.status.success() && !answer.is_empty()
    {
        true => Some(answer),
        false => None
    }
}

/// The executable file a program name refers to, or nothing.
///
/// A name holding a separator is taken as a location and checked; a bare name is
/// looked up along the login shell's `PATH`. Either way the answer is a file
/// with an execute bit, because that is the only thing a pseudo-terminal can
/// spawn.
pub fn resolve_program(name: &str) -> Option<PathBuf>
{
    if name.is_empty()
    {
        return None;
    }

    if name.contains('/')
    {
        let path = std::fs::canonicalize(name).ok()?;
        return executable(&path).then_some(path);
    }

    login_path()?
        .split(':')
        .filter(|directory| !directory.is_empty())
        .map(|directory| PathBuf::from(directory).join(name))
        .find(|candidate| executable(candidate))
}

/// Whether this path is a file something can be spawned from.
fn executable(path: &Path) -> bool
{
    let Ok(meta) = std::fs::metadata(path)
    else
    {
        return false;
    };

    meta.is_file() && meta.permissions().mode() & 0o111 != 0
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn an_absolute_path_is_taken_as_given()
    {
        assert_eq!(
            resolve_program("/bin/echo"),
            Some(PathBuf::from("/bin/echo"))
        );
    }

    /// The regression this module was rewritten for during the spike: the login
    /// shell answers `echo` for `echo`, and a shell builtin is not a spawnable
    /// executable.
    #[test]
    fn a_bare_name_resolves_to_an_executable_file_not_a_shell_builtin()
    {
        let found = resolve_program("echo").expect("no `echo` executable is on the login PATH");

        assert!(found.is_absolute(), "{}", found.display());
        assert!(executable(&found));
    }

    #[test]
    fn a_name_that_does_not_exist_resolves_to_nothing()
    {
        assert!(resolve_program("trdr-no-such-executable-9c1f").is_none());
        assert!(resolve_program("").is_none());
    }

    /// The name never reaches a shell as source. If it did, this would run `id`.
    #[test]
    fn a_name_carrying_shell_syntax_cannot_run_a_second_command()
    {
        assert!(resolve_program("nope\"; id; echo \"").is_none());
        assert!(resolve_program("$(id)").is_none());
    }

    /// A directory on the `PATH` shares its name with nothing spawnable, and a
    /// file without an execute bit is not a program however well its name
    /// matches.
    #[test]
    fn a_directory_or_a_plain_file_is_not_accepted_as_a_program()
    {
        assert!(!executable(Path::new("/bin")));
        assert!(!executable(Path::new("/etc/hosts")));
    }

    /// The two answers this module gives about the environment, checked for
    /// shape rather than for content: what they hold is the machine's, and a
    /// test that named a directory would only be describing this machine.
    #[test]
    fn the_login_shell_and_its_path_are_answered_without_being_invented()
    {
        assert!(login_shell().starts_with('/'));

        if let Some(path) = login_path()
        {
            assert!(path.split(':').any(|entry| entry.starts_with('/')));
        }
    }
}
