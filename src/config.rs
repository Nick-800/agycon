use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultFilter {
    Current,
    All,
}

impl std::fmt::Display for DefaultFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DefaultFilter::Current => write!(f, "current"),
            DefaultFilter::All => write!(f, "all"),
        }
    }
}

impl std::str::FromStr for DefaultFilter {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "current" | "c" | "filtered" => Ok(DefaultFilter::Current),
            "all" | "a" => Ok(DefaultFilter::All),
            other => Err(format!(
                "Invalid filter mode '{}'. Expected 'current' or 'all'",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub default_filter: DefaultFilter,
    #[serde(default)]
    pub pinned_ids: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_filter: DefaultFilter::Current,
            pinned_ids: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn is_pinned(&self, id: &str) -> bool {
        self.pinned_ids.iter().any(|p| p == id)
    }

    pub fn toggle_pin(&mut self, id: &str) -> bool {
        if let Some(pos) = self.pinned_ids.iter().position(|p| p == id) {
            self.pinned_ids.remove(pos);
            false
        } else {
            self.pinned_ids.push(id.to_string());
            true
        }
    }
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("agycon").join("config.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(path) = Self::config_path() else {
            return Err("Unable to determine config directory".into());
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
