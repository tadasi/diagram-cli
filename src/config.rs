use std::env;
use std::fs;
use std::path::PathBuf;

pub const DIAGRAM_TYPES: &[(&str, &str)] = &[
    ("flowchart", "フローチャート"),
    ("sequence", "シーケンス図"),
    ("activity", "アクティビティ図"),
    ("component", "コンポーネント図"),
    ("state", "状態遷移図"),
];

pub struct DgConfig {
    pub workspace: String,
    pub diagram_type: String,
    pub output_dir: String,
}

impl DgConfig {
    fn config_path() -> Option<PathBuf> {
        let home = env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".config/dg/config.json"))
    }

    pub fn load() -> Option<DgConfig> {
        let path = Self::config_path()?;
        let text = fs::read_to_string(path).ok()?;
        Self::from_json(&text)
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("could not resolve config path")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        fs::write(&path, self.to_json()).map_err(|e| format!("write: {e}"))
    }

    pub fn workspace_abs(&self) -> PathBuf {
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

    fn to_json(&self) -> String {
        format!(
            "{{\n  \"workspace\": \"{}\",\n  \"diagram_type\": \"{}\",\n  \"output_dir\": \"{}\"\n}}\n",
            self.workspace, self.diagram_type, self.output_dir
        )
    }

    fn from_json(text: &str) -> Option<DgConfig> {
        let ws = json_string_value(text, "workspace")?;
        let dt = json_string_value(text, "diagram_type")?;
        let od = json_string_value(text, "output_dir")?;
        Some(DgConfig { workspace: ws, diagram_type: dt, output_dir: od })
    }
}

fn json_string_value(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let i = json.find(&needle)? + needle.len();
    let rest = &json[i..];
    let colon = rest.find(':')?;
    let after = &rest[colon + 1..];
    let q1 = after.find('"')? + 1;
    let after_q = &after[q1..];
    let q2 = after_q.find('"')?;
    Some(after_q[..q2].to_string())
}

pub fn home_dir() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_default())
}
