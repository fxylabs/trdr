use std::path::Path;

use rusqlite::Connection;

use crate::credential::CredentialState;
use crate::kis::AccountSnapshot;

pub struct Store
{
    connection: Connection
}

impl Store
{
    pub fn open(path: &Path) -> rusqlite::Result<Store>
    {
        let connection = Connection::open(path)?;
        // WAL because that is what the product will run, and because it means
        // the `-wal` and `-shm` files exist to be scanned. A secret that reached
        // the database is in the write-ahead log before it is in the main file.
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS account_snapshot (
                 id              INTEGER PRIMARY KEY CHECK (id = 1),
                 total_evaluated INTEGER NOT NULL,
                 cash            INTEGER NOT NULL,
                 holdings        TEXT    NOT NULL
             );
             CREATE TABLE IF NOT EXISTS collector_credential (
                 collector TEXT PRIMARY KEY,
                 state     TEXT NOT NULL
             );"
        )?;
        Ok(Store { connection })
    }

    /// One row, replaced each time. Section 5.2: v0 keeps the latest normalized
    /// snapshot and does not keep the ones before it, so there is no history for
    /// an account number to hide in.
    pub fn save_snapshot(&self, snapshot: &AccountSnapshot) -> rusqlite::Result<()>
    {
        let holdings = serde_json::to_string(&snapshot.holdings).unwrap_or_else(|_| "[]".to_string());
        self.connection.execute(
            "INSERT INTO account_snapshot (id, total_evaluated, cash, holdings) VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET total_evaluated = ?1, cash = ?2, holdings = ?3",
            rusqlite::params![snapshot.total_evaluated, snapshot.cash, holdings]
        )?;
        Ok(())
    }

    pub fn latest_snapshot(&self) -> rusqlite::Result<Option<AccountSnapshot>>
    {
        let mut statement = self.connection.prepare("SELECT total_evaluated, cash, holdings FROM account_snapshot WHERE id = 1")?;
        let mut rows = statement.query([])?;
        let Some(row) = rows.next()? else { return Ok(None) };
        let holdings: String = row.get(2)?;
        Ok(Some(AccountSnapshot
        {
            total_evaluated: row.get(0)?,
            cash: row.get(1)?,
            holdings: serde_json::from_str(&holdings).unwrap_or_default()
        }))
    }

    /// The state, and there is no column for a value to go in.
    pub fn save_credential_state(&self, collector: &str, state: CredentialState) -> rusqlite::Result<()>
    {
        let state = serde_json::to_string(&state).unwrap_or_default();
        self.connection.execute(
            "INSERT INTO collector_credential (collector, state) VALUES (?1, ?2)
             ON CONFLICT(collector) DO UPDATE SET state = ?2",
            rusqlite::params![collector, state.trim_matches('"')]
        )?;
        Ok(())
    }

    /// Flushes the write-ahead log into the main file, so a scan afterwards is
    /// reading everything the database holds rather than only the settled part.
    pub fn checkpoint(&self) -> rusqlite::Result<()>
    {
        self.connection.pragma_update(None, "wal_checkpoint", "TRUNCATE")
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::kis::Holding;

    const CANARY: &str = "TRDR-CANARY-6f2a9c";

    fn scratch(name: &str) -> std::path::PathBuf
    {
        let directory = std::env::temp_dir().join(format!("trdr-spike-store-{name}"));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the scratch directory could not be made");
        directory.join("trdr.sqlite3")
    }

    fn snapshot() -> AccountSnapshot
    {
        AccountSnapshot
        {
            total_evaluated: 2_560_000,
            cash: 1_000_000,
            holdings: vec![Holding
            {
                symbol: "005930".to_string(),
                name: "Samsung Electronics".to_string(),
                quantity: 12,
                evaluated: 948_000
            }]
        }
    }

    #[test]
    fn a_snapshot_survives_a_round_trip()
    {
        let path = scratch("round-trip");
        let store = Store::open(&path).expect("the database should open");
        store.save_snapshot(&snapshot()).expect("the snapshot should save");
        assert_eq!(store.latest_snapshot().expect("it should read"), Some(snapshot()));
    }

    // Section 5.2 keeps the latest snapshot only. Two saves must leave one row,
    // not a history a later reader could mine.
    #[test]
    fn a_second_snapshot_replaces_the_first_rather_than_joining_it()
    {
        let path = scratch("single-row");
        let store = Store::open(&path).expect("the database should open");
        store.save_snapshot(&snapshot()).expect("the snapshot should save");

        let mut second = snapshot();
        second.cash = 7;
        store.save_snapshot(&second).expect("the snapshot should save");

        let rows: i64 = store
            .connection
            .query_row("SELECT count(*) FROM account_snapshot", [], |row| row.get(0))
            .expect("the count should read");
        assert_eq!(rows, 1);
        assert_eq!(store.latest_snapshot().expect("it should read").expect("a row is expected").cash, 7);
    }

    #[test]
    fn a_credential_row_holds_a_state_and_has_nowhere_to_put_a_value()
    {
        let path = scratch("credential-state");
        let store = Store::open(&path).expect("the database should open");
        store.save_credential_state("kis-spike", CredentialState::RateLimited).expect("the state should save");

        let state: String = store
            .connection
            .query_row("SELECT state FROM collector_credential WHERE collector = 'kis-spike'", [], |row| row.get(0))
            .expect("the state should read");
        assert_eq!(state, "rate-limited");

        let columns: Vec<String> = store
            .connection
            .prepare("SELECT name FROM pragma_table_info('collector_credential')")
            .expect("the schema should read")
            .query_map([], |row| row.get(0))
            .expect("the schema should read")
            .filter_map(Result::ok)
            .collect();
        assert_eq!(columns, vec!["collector".to_string(), "state".to_string()]);
    }

    #[test]
    fn nothing_in_the_database_file_carries_the_canary()
    {
        let path = scratch("no-canary");
        let store = Store::open(&path).expect("the database should open");
        store.save_snapshot(&snapshot()).expect("the snapshot should save");
        store.save_credential_state("kis-spike", CredentialState::Valid).expect("the state should save");
        store.checkpoint().expect("the write-ahead log should flush");

        let bytes = std::fs::read(&path).expect("the database file should read");
        assert!(!bytes.windows(CANARY.len()).any(|window| window == CANARY.as_bytes()));
    }
}
