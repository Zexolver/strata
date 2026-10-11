use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    pub adblock: bool,
    /// Experimental: isolated profiles. Off by default.
    pub profiles_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { theme: Theme::System, adblock: true, profiles_enabled: false }
    }
}

impl Settings {
    pub fn load(path: &std::path::Path) -> Self {
        std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_keep_experiment_off() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert!(!s.profiles_enabled && s.adblock && s.theme == Theme::System);
    }
}
