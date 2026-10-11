use std::path::Path;
use std::time::{Duration, SystemTime};
use strata_core::blocker::Blocker;

const LISTS: &[(&str, &str)] = &[
    ("easylist.txt", "https://easylist.to/easylist/easylist.txt"),
    ("easyprivacy.txt", "https://easylist.to/easylist/easyprivacy.txt"),
    ("ublock-filters.txt", "https://ublockorigin.github.io/uAssets/filters/filters.txt"),
    ("brave-unbreak.txt", "https://raw.githubusercontent.com/brave/adblock-lists/master/brave-unbreak.txt"),
];
const MAX_AGE: Duration = Duration::from_secs(3 * 24 * 3600);

fn read_cached(dir: &Path) -> Vec<String> {
    LISTS.iter().filter_map(|(f, _)| std::fs::read_to_string(dir.join(f)).ok()).collect()
}

fn stale(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| SystemTime::now().duration_since(t).unwrap_or_default() > MAX_AGE)
        .unwrap_or(true)
}

#[cfg(feature = "updater")]
fn download(url: &str) -> Option<String> {
    ureq::get(url).call().ok()?.body_mut().read_to_string().ok()
}

#[cfg(not(feature = "updater"))]
fn download(_url: &str) -> Option<String> {
    None
}

/// Load cached lists immediately, then refresh anything older than three days.
/// Runs on its own thread so start-up never waits on the network.
pub fn spawn_updater(blocker: std::sync::Arc<Blocker>, dir: std::path::PathBuf) {
    std::thread::spawn(move || {
        let _ = std::fs::create_dir_all(&dir);
        let cached = read_cached(&dir);
        if !cached.is_empty() {
            blocker.load_lists(cached);
        }
        let mut changed = false;
        for (file, url) in LISTS {
            let path = dir.join(file);
            if stale(&path) {
                if let Some(text) = download(url) {
                    if std::fs::write(&path, text).is_ok() {
                        changed = true;
                    }
                }
            }
        }
        if changed || !blocker.is_loaded() {
            let lists = read_cached(&dir);
            if !lists.is_empty() {
                blocker.load_lists(lists);
            }
        }
    });
}
