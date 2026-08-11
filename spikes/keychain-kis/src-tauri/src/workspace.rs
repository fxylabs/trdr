use std::fs;
use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The product root. `TRDR_SPIKE_HOME` moves it so a test never writes into the
/// user's live `~/.trdr`.
pub fn product_root() -> PathBuf
{
    if let Ok(overridden) = std::env::var("TRDR_SPIKE_HOME")
    {
        return PathBuf::from(overridden);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".trdr")
}

/// The spike keeps out of `workspaces/default`, which is where the product's own
/// state will live. Everything this spike writes is under a name that says what
/// wrote it and can be deleted with the rest of M1.
pub const SPIKE_WORKSPACE: &str = "spike-keychain-kis";

#[derive(Debug, Serialize, Deserialize)]
struct Config
{
    /// Section 5.1: this identifies the Mac, not the workspace, which is why a
    /// restored backup on another machine does not inherit this one's Keychain
    /// items by name collision.
    local_instance_id: String
}

/// Reads the local instance id, creating one on first run.
pub fn local_instance_id(root: &Path) -> io::Result<String>
{
    let path = root.join("config.json");
    if let Ok(text) = fs::read_to_string(&path)
    {
        if let Ok(config) = serde_json::from_str::<Config>(&text)
        {
            return Ok(config.local_instance_id);
        }
    }

    fs::create_dir_all(root)?;
    fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
    let config = Config { local_instance_id: random_id()? };
    let text = serde_json::to_string_pretty(&config).map_err(io::Error::other)?;
    fs::write(&path, text)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    Ok(config.local_instance_id)
}

fn random_id() -> io::Result<String>
{
    let mut bytes = [0u8; 16];
    fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub struct Workspace
{
    root: PathBuf
}

impl Workspace
{
    pub fn open(product_root: &Path) -> io::Result<Workspace>
    {
        let root = product_root.join("workspaces").join(SPIKE_WORKSPACE);
        for directory in [&root, &root.join("logs"), &root.join("exports")]
        {
            fs::create_dir_all(directory)?;
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
        }
        Ok(Workspace { root })
    }

    pub fn root(&self) -> &Path
    {
        &self.root
    }

    pub fn db_path(&self) -> PathBuf
    {
        self.root.join("trdr.sqlite3")
    }

    pub fn log_path(&self) -> PathBuf
    {
        self.root.join("logs").join("app.log")
    }

    pub fn export_path(&self) -> PathBuf
    {
        self.root.join("exports").join("today.json")
    }

    /// Every file this spike writes, which is also every file the canary scan
    /// has to cover. Keeping the two derived from one list is the point: a new
    /// artifact that is not scanned is the failure mode the scan exists to
    /// catch, and a second hand-written list would drift from this one.
    pub fn artifacts(&self) -> Vec<PathBuf>
    {
        let database = self.db_path();
        vec![
            self.log_path(),
            database.clone(),
            database.with_extension("sqlite3-wal"),
            database.with_extension("sqlite3-shm"),
            self.export_path()
        ]
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn scratch(name: &str) -> PathBuf
    {
        let directory = std::env::temp_dir().join(format!("trdr-spike-keychain-{name}"));
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    #[test]
    fn a_local_instance_id_is_made_once_and_then_kept()
    {
        let root = scratch("instance-id");
        let first = local_instance_id(&root).expect("an id should be creatable");
        let second = local_instance_id(&root).expect("the id should be readable");
        assert_eq!(first, second, "a second run invented a new instance id");
        assert_eq!(first.len(), 32);
    }

    #[test]
    fn the_config_file_is_readable_only_by_its_owner()
    {
        let root = scratch("config-mode");
        local_instance_id(&root).expect("an id should be creatable");
        let mode = fs::metadata(root.join("config.json")).expect("config.json should exist").permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn the_workspace_directories_are_private_to_their_owner()
    {
        let root = scratch("workspace-mode");
        let workspace = Workspace::open(&root).expect("the workspace should be creatable");
        for directory in [workspace.root().to_path_buf(), workspace.root().join("logs"), workspace.root().join("exports")]
        {
            let mode = fs::metadata(&directory).expect("the directory should exist").permissions().mode();
            assert_eq!(mode & 0o777, 0o700, "{} is not private", directory.display());
        }
    }

    #[test]
    fn the_spike_stays_out_of_the_default_workspace()
    {
        let root = scratch("workspace-name");
        let workspace = Workspace::open(&root).expect("the workspace should be creatable");
        assert!(workspace.root().ends_with(SPIKE_WORKSPACE));
        assert!(!root.join("workspaces").join("default").exists());
    }

    #[test]
    fn the_artifact_list_covers_the_database_and_its_sidecars()
    {
        let root = scratch("artifacts");
        let workspace = Workspace::open(&root).expect("the workspace should be creatable");
        let named: Vec<String> = workspace.artifacts().iter().map(|path| path.display().to_string()).collect();
        for required in ["app.log", "trdr.sqlite3", "trdr.sqlite3-wal", "trdr.sqlite3-shm", "today.json"]
        {
            assert!(named.iter().any(|path| path.ends_with(required)), "{required} is not scanned");
        }
    }
}
