use crate::nav;
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
    /// Id from `nav::ENGINES`, or "custom" to use `custom_search`.
    pub search_engine: String,
    pub custom_search: String,
    /// Minutes a background tab may sit idle before its page is unloaded to free memory (0 = never).
    pub discard_minutes: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            adblock: true,
            profiles_enabled: false,
            search_engine: nav::DEFAULT_ENGINE.into(),
            custom_search: String::new(),
            discard_minutes: 5,
        }
    }
}

impl Settings {
    /// The search URL template in effect (falls back to the default engine if custom is invalid).
    pub fn search_template(&self) -> String {
        if self.search_engine == "custom" && nav::valid_template(&self.custom_search) {
            return self.custom_search.clone();
        }
        nav::engine(&self.search_engine).unwrap_or_else(|| nav::engine(nav::DEFAULT_ENGINE).unwrap()).template.into()
    }

    /// Where a new tab opens: the search provider's home page.
    pub fn home_url(&self) -> String {
        if self.search_engine == "custom" && nav::valid_template(&self.custom_search) {
            if let Ok(mut u) = url::Url::parse(&self.custom_search.replace("%s", "")) {
                u.set_path("/");
                u.set_query(None);
                return u.into();
            }
        }
        nav::engine(&self.search_engine).unwrap_or_else(|| nav::engine(nav::DEFAULT_ENGINE).unwrap()).home.into()
    }

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
        assert_eq!(s.home_url(), "https://duckduckgo.com/");
    }

    #[test]
    fn custom_engine_home() {
        let s = Settings { search_engine: "custom".into(), custom_search: "https://s.example.org/find/?q=%s".into(), ..Default::default() };
        assert_eq!(s.home_url(), "https://s.example.org/");
        assert_eq!(s.search_template(), "https://s.example.org/find/?q=%s");
        let bad = Settings { search_engine: "custom".into(), custom_search: "nope".into(), ..Default::default() };
        assert_eq!(bad.home_url(), "https://duckduckgo.com/");
    }
}
