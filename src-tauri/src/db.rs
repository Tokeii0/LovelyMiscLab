//! SQLite persistence via rusqlite. Relational project data (projects, files,
//! reports, findings, artifacts, edges) lives here; large artifact bytes live on
//! disk with only their path in the DB. App settings use the store plugin instead.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let mut conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Self::migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// In-memory database, for tests.
    #[allow(dead_code)]
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        Self::migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("db mutex poisoned")
    }

    fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < 1 {
            let tx = conn.transaction()?;
            tx.execute_batch(MIGRATION_V1)?;
            tx.pragma_update(None, "user_version", 1)?;
            tx.commit()?;
        }
        Ok(())
    }
}

const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    created     INTEGER NOT NULL,
    opened_at   INTEGER,
    meta_json   TEXT
);

CREATE TABLE IF NOT EXISTS files (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  INTEGER REFERENCES projects(id) ON DELETE CASCADE,
    path        TEXT    NOT NULL,
    name        TEXT    NOT NULL,
    size        INTEGER NOT NULL,
    md5         TEXT,
    sha256      TEXT,
    ssdeep      TEXT,
    mime        TEXT,
    file_type   TEXT,
    imported_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS reports (
    file_id          INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    suspicion        INTEGER,
    clue_count       INTEGER,
    elapsed_ms       INTEGER,
    fingerprint_json TEXT,
    entropy_json     TEXT
);

CREATE TABLE IF NOT EXISTS findings (
    id           TEXT PRIMARY KEY,
    file_id      INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    source       TEXT,
    stage        INTEGER,
    severity     INTEGER,
    confidence   REAL,
    score_weight REAL,
    tags_json    TEXT,
    title        TEXT,
    detail       TEXT,
    evidence_json TEXT
);

CREATE TABLE IF NOT EXISTS artifacts (
    id              TEXT PRIMARY KEY,
    file_id         INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    kind            TEXT,
    label           TEXT,
    mime            TEXT,
    size            INTEGER,
    store_kind      TEXT,
    store_locator   TEXT,
    child_report_id TEXT
);

CREATE TABLE IF NOT EXISTS artifact_blobs (
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    bytes BLOB
);

CREATE TABLE IF NOT EXISTS edges (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER REFERENCES projects(id) ON DELETE CASCADE,
    from_ref   TEXT,
    to_ref     TEXT,
    relation   TEXT,
    weight     REAL
);

CREATE INDEX IF NOT EXISTS idx_files_project    ON files(project_id);
CREATE INDEX IF NOT EXISTS idx_findings_file    ON findings(file_id);
CREATE INDEX IF NOT EXISTS idx_artifacts_file   ON artifacts(file_id);
CREATE INDEX IF NOT EXISTS idx_edges_project    ON edges(project_id);
"#;
