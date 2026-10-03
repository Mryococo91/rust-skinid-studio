//! Local project storage and publish history (SQLite).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{RssError, RssResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: String,
    pub title: String,
    pub description: String,
    pub item_type: String,
    pub asset_dir: String,
    pub published_file_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishHistoryEntry {
    pub id: String,
    pub project_id: String,
    pub published_file_id: String,
    pub url: String,
    pub dry_run: bool,
    pub success: bool,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

pub struct ProjectStore {
    db_path: PathBuf,
    conn: Connection,
}

impl ProjectStore {
    pub fn open(db_path: &Path) -> RssResult<Self> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        let store = Self {
            db_path: db_path.to_path_buf(),
            conn,
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    fn migrate(&self) -> RssResult<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                item_type TEXT NOT NULL,
                asset_dir TEXT NOT NULL,
                published_file_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS publish_history (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                published_file_id TEXT NOT NULL,
                url TEXT NOT NULL,
                dry_run INTEGER NOT NULL,
                success INTEGER NOT NULL,
                message TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id)
            );
            "#,
        )?;
        Ok(())
    }

    pub fn upsert_project(&self, project: &ProjectRecord) -> RssResult<()> {
        self.conn.execute(
            r#"
            INSERT INTO projects (id, title, description, item_type, asset_dir, published_file_id, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                title=excluded.title,
                description=excluded.description,
                item_type=excluded.item_type,
                asset_dir=excluded.asset_dir,
                published_file_id=excluded.published_file_id,
                updated_at=excluded.updated_at
            "#,
            params![
                project.id,
                project.title,
                project.description,
                project.item_type,
                project.asset_dir,
                project.published_file_id,
                project.created_at.to_rfc3339(),
                project.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list_projects(&self) -> RssResult<Vec<ProjectRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, item_type, asset_dir, published_file_id, created_at, updated_at
             FROM projects ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProjectRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                item_type: row.get(3)?,
                asset_dir: row.get(4)?,
                published_file_id: row.get(5)?,
                created_at: parse_dt(row.get::<_, String>(6)?),
                updated_at: parse_dt(row.get::<_, String>(7)?),
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn get_project(&self, id: &str) -> RssResult<Option<ProjectRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, item_type, asset_dir, published_file_id, created_at, updated_at
             FROM projects WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ProjectRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                item_type: row.get(3)?,
                asset_dir: row.get(4)?,
                published_file_id: row.get(5)?,
                created_at: parse_dt(row.get::<_, String>(6)?),
                updated_at: parse_dt(row.get::<_, String>(7)?),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn add_history(&self, entry: &PublishHistoryEntry) -> RssResult<()> {
        self.conn.execute(
            r#"
            INSERT INTO publish_history
            (id, project_id, published_file_id, url, dry_run, success, message, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                entry.id,
                entry.project_id,
                entry.published_file_id,
                entry.url,
                entry.dry_run as i32,
                entry.success as i32,
                entry.message,
                entry.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list_history(&self, project_id: Option<&str>) -> RssResult<Vec<PublishHistoryEntry>> {
        if let Some(pid) = project_id {
            let mut stmt = self.conn.prepare(
                "SELECT id, project_id, published_file_id, url, dry_run, success, message, created_at
                 FROM publish_history WHERE project_id = ?1 ORDER BY created_at DESC",
            )?;
            let rows = stmt.query_map(params![pid], map_history)?;
            collect_history(rows)
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, project_id, published_file_id, url, dry_run, success, message, created_at
                 FROM publish_history ORDER BY created_at DESC",
            )?;
            let rows = stmt.query_map([], map_history)?;
            collect_history(rows)
        }
    }

    pub fn new_project_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Wipe all projects and publish history from the database.
    pub fn wipe_all(&self) -> RssResult<()> {
        self.conn
            .execute_batch("DELETE FROM publish_history; DELETE FROM projects;")?;
        Ok(())
    }

    /// Local app data directory.
    ///
    /// Priority:
    /// 1. `RSS_DATA_DIR` env override
    /// 2. Portable mode (`PORTABLE` marker next to the exe, or `RSS_PORTABLE=1`) → `./app-data`
    /// 3. Installed mode → `%LOCALAPPDATA%\dev.rss.skinid-studio` (matches Tauri bundle id)
    pub fn default_data_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("RSS_DATA_DIR") {
            return PathBuf::from(dir);
        }
        let exe_dir = executable_dir();
        if is_portable(&exe_dir) {
            return exe_dir.join("app-data");
        }
        dirs_data_local().join(APP_DATA_FOLDER)
    }
}

/// Bundle identifier / installed AppData folder name (must stay in sync with tauri.conf.json).
pub const APP_DATA_FOLDER: &str = "dev.rss.skinid-studio";

fn executable_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn is_portable(exe_dir: &Path) -> bool {
    exe_dir.join("PORTABLE").is_file() || std::env::var_os("RSS_PORTABLE").is_some()
}

fn map_history(row: &rusqlite::Row<'_>) -> rusqlite::Result<PublishHistoryEntry> {
    Ok(PublishHistoryEntry {
        id: row.get(0)?,
        project_id: row.get(1)?,
        published_file_id: row.get(2)?,
        url: row.get(3)?,
        dry_run: row.get::<_, i32>(4)? != 0,
        success: row.get::<_, i32>(5)? != 0,
        message: row.get(6)?,
        created_at: parse_dt(row.get::<_, String>(7)?),
    })
}

fn collect_history(
    rows: impl Iterator<Item = rusqlite::Result<PublishHistoryEntry>>,
) -> RssResult<Vec<PublishHistoryEntry>> {
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(RssError::from)?);
    }
    Ok(out)
}

fn parse_dt(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn dirs_data_local() -> PathBuf {
    if let Ok(p) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(p);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local").join("share");
    }
    PathBuf::from(".").join("data")
}

pub fn create_project_dirs(root: &Path, project_id: &str) -> RssResult<PathBuf> {
    let dir = root.join("projects").join(project_id);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn data_dir_respects_env_override() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("custom-data");
        std::env::set_var("RSS_DATA_DIR", &path);
        assert_eq!(ProjectStore::default_data_dir(), path);
        std::env::remove_var("RSS_DATA_DIR");
    }

    #[test]
    fn project_roundtrip_and_history() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("test.db");
        let store = ProjectStore::open(&db).unwrap();
        let id = ProjectStore::new_project_id();
        let now = Utc::now();
        let project = ProjectRecord {
            id: id.clone(),
            title: "Test".into(),
            description: "Desc".into(),
            item_type: "Grenade".into(),
            asset_dir: dir.path().join("asset").to_string_lossy().into(),
            published_file_id: None,
            created_at: now,
            updated_at: now,
        };
        store.upsert_project(&project).unwrap();
        let listed = store.list_projects().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title, "Test");

        store
            .add_history(&PublishHistoryEntry {
                id: Uuid::new_v4().to_string(),
                project_id: id.clone(),
                published_file_id: "9000000001".into(),
                url: "https://example".into(),
                dry_run: true,
                success: true,
                message: "ok".into(),
                created_at: Utc::now(),
            })
            .unwrap();
        let hist = store.list_history(Some(&id)).unwrap();
        assert_eq!(hist.len(), 1);
    }
}
