use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_ID: &str = "default";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    /// Accent colour shown on this profile's tabs, e.g. "#7aa2f7".
    pub color: String,
    /// Optional per-profile proxy (e.g. `socks5://127.0.0.1:9050` for Tor, or an HTTP proxy).
    pub proxy: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProfileStore {
    pub profiles: Vec<Profile>,
}

#[derive(thiserror::Error, Debug)]
pub enum ProfileError {
    #[error("profile name must not be empty")]
    EmptyName,
    #[error("cannot delete the default profile")]
    DeleteDefault,
    #[error("no such profile")]
    NotFound,
    #[error("invalid proxy URL (use http://, https://, socks5://)")]
    BadProxy,
}

impl Default for ProfileStore {
    fn default() -> Self {
        Self { profiles: vec![Profile { id: DEFAULT_ID.into(), name: "Default".into(), color: "#7aa2f7".into(), proxy: None }] }
    }
}

fn valid_proxy(p: &str) -> bool {
    url::Url::parse(p).map_or(false, |u| matches!(u.scheme(), "http" | "https" | "socks5") && u.host().is_some())
}

impl ProfileStore {
    pub fn load(path: &Path) -> Self {
        let mut s: Self = std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        if !s.profiles.iter().any(|p| p.id == DEFAULT_ID) {
            s.profiles.insert(0, Self::default().profiles.remove(0));
        }
        s
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self).unwrap())
    }

    pub fn get(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn add(&mut self, name: &str, color: &str, proxy: Option<String>) -> Result<Profile, ProfileError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(ProfileError::EmptyName);
        }
        let proxy = proxy.map(|p| p.trim().to_string()).filter(|p| !p.is_empty());
        if proxy.as_deref().map_or(false, |p| !valid_proxy(p)) {
            return Err(ProfileError::BadProxy);
        }
        let mut id: String = name.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
        if id.is_empty() {
            id = "profile".into();
        }
        let base = id.clone();
        let mut n = 2;
        while self.get(&id).is_some() {
            id = format!("{base}{n}");
            n += 1;
        }
        let p = Profile { id, name: name.into(), color: color.into(), proxy };
        self.profiles.push(p.clone());
        Ok(p)
    }

    pub fn remove(&mut self, id: &str) -> Result<(), ProfileError> {
        if id == DEFAULT_ID {
            return Err(ProfileError::DeleteDefault);
        }
        let before = self.profiles.len();
        self.profiles.retain(|p| p.id != id);
        if self.profiles.len() == before { Err(ProfileError::NotFound) } else { Ok(()) }
    }

    /// Each profile gets its own web-engine data directory (cookies, storage, cache),
    /// which is what actually keeps profiles invisible to each other.
    pub fn data_dir(base: &Path, id: &str) -> PathBuf {
        base.join("profiles").join(id).join("webview")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_remove_and_unique_ids() {
        let mut s = ProfileStore::default();
        let a = s.add("School", "#fff", None).unwrap();
        let b = s.add("School", "#000", None).unwrap();
        assert_eq!((a.id.as_str(), b.id.as_str()), ("school", "school2"));
        assert!(s.remove("school").is_ok());
        assert!(matches!(s.remove("default"), Err(ProfileError::DeleteDefault)));
        assert!(matches!(s.add("  ", "#fff", None), Err(ProfileError::EmptyName)));
    }

    #[test]
    fn proxy_validation_and_path_safety() {
        let mut s = ProfileStore::default();
        assert!(s.add("Tor", "#fff", Some("socks5://127.0.0.1:9050".into())).is_ok());
        assert!(matches!(s.add("X", "#fff", Some("ftp://x".into())), Err(ProfileError::BadProxy)));
        let p = s.add("../../etc", "#fff", None).unwrap();
        assert_eq!(p.id, "etc");
    }
}
