use std::path::PathBuf;
use std::sync::Mutex;

use rss_core::project::ProjectStore;

pub struct AppState {
    pub store: Mutex<ProjectStore>,
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        let data_dir = ProjectStore::default_data_dir();
        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        let db = data_dir.join("studio.db");
        let store = ProjectStore::open(&db).map_err(|e| e.to_string())?;
        Ok(Self {
            store: Mutex::new(store),
            data_dir,
        })
    }
}
