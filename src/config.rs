use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const DIAGRAM_TYPES: &[(&str, &str)] = &[
    ("flowchart", "フローチャート"),
    ("sequence", "シーケンス図"),
];

#[derive(Serialize, Deserialize)]
pub struct DgConfig {
    pub workspace: String,
    pub diagram_type: String,
    pub output_dir: String,
}

impl DgConfig {
    fn config_path() -> Result<PathBuf> {
        let home = env::var("HOME").context("HOME not set")?;
        Ok(PathBuf::from(home).join(".config/dg/config.json"))
    }

    pub fn load() -> Option<DgConfig> {
        let path = Self::config_path().ok()?;
        let text = fs::read_to_string(path).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;
        Ok(())
    }

    pub fn workspace_full_path(&self) -> PathBuf {
        home_dir().join(&self.workspace)
    }

    pub fn output_dir_abs(&self) -> PathBuf {
        home_dir().join(&self.output_dir)
    }

    pub fn diagram_type_label(&self) -> &str {
        DIAGRAM_TYPES
            .iter()
            .find(|(k, _)| *k == self.diagram_type)
            .map(|(_, v)| *v)
            .unwrap_or("フローチャート")
    }
}

pub fn home_dir() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_default())
}
