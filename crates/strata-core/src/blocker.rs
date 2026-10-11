use adblock::cosmetic_filter_cache::UrlSpecificResources;
use adblock::lists::{FilterSet, ParseOptions};
use adblock::request::Request;
use adblock::Engine;
use std::sync::{Arc, RwLock};

/// Thread-safe holder around Brave's `adblock` engine; the engine can be swapped
/// atomically when lists are refreshed.
#[derive(Default)]
pub struct Blocker {
    engine: RwLock<Option<Arc<Engine>>>,
}

impl Blocker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compile the given filter-list texts into a new engine and install it.
    pub fn load_lists<I: IntoIterator<Item = String>>(&self, lists: I) {
        let mut set = FilterSet::new(false);
        for text in lists {
            set.add_filter_list(text, ParseOptions::default());
        }
        let engine = Arc::new(Engine::new_with_filter_set(set));
        *self.engine.write().unwrap() = Some(engine);
    }

    pub fn is_loaded(&self) -> bool {
        self.engine.read().unwrap().is_some()
    }

    fn engine(&self) -> Option<Arc<Engine>> {
        self.engine.read().unwrap().clone()
    }

    /// Should this request be blocked? `request_type` uses adblock's names
    /// ("document", "script", "image", "xmlhttprequest", ...).
    pub fn should_block(&self, url: &str, source_url: &str, request_type: &str) -> bool {
        let Some(engine) = self.engine() else { return false };
        let Ok(req) = Request::new(url, source_url, request_type, "GET") else { return false };
        let r = engine.check_network_request(&req);
        r.important || (r.filter.is_some() && r.exception.is_none()) || r.redirect.is_some()
    }

    /// JavaScript to run in a page at `url`: hides ad elements and runs scriptlets.
    /// Returns `None` when nothing applies.
    pub fn cosmetic_script(&self, url: &str) -> Option<String> {
        let engine = self.engine()?;
        let res = engine.url_cosmetic_resources(url);
        build_cosmetic_script(&res)
    }
}

fn build_cosmetic_script(res: &UrlSpecificResources) -> Option<String> {
    if res.hide_selectors.is_empty() && res.injected_script.is_empty() {
        return None;
    }
    let mut css = String::new();
    for sel in &res.hide_selectors {
        css.push_str(sel);
        css.push_str("{display:none!important}\n");
    }
    let css_lit = serde_json::to_string(&css).unwrap();
    let mut js = String::from("(()=>{try{");
    if !css.is_empty() {
        js.push_str(&format!(
            "const s=document.createElement('style');s.textContent={css_lit};\
             (document.head||document.documentElement).appendChild(s);"
        ));
    }
    js.push_str("}catch(e){}");
    if !res.injected_script.is_empty() {
        js.push_str("try{");
        js.push_str(&res.injected_script);
        js.push_str("\n}catch(e){}");
    }
    js.push_str("})();");
    Some(js)
}

/// Convert filter lists to WebKit content-blocking JSON (for WebKitGTK / WKWebView,
/// which only accept declarative rules rather than per-request callbacks).
pub fn content_blocking_json<I: IntoIterator<Item = String>>(lists: I) -> Result<String, String> {
    let mut set = FilterSet::new(true);
    for text in lists {
        set.add_filter_list(text, ParseOptions::default());
    }
    let (rules, _) = set.into_content_blocking().map_err(|_| "conversion failed".to_string())?;
    serde_json::to_string(&rules).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "||ads.example.com^\n||tracker.net^$third-party\n@@||tracker.net/ok.js\n##.banner-ad\nexample.org##.promo\n";

    fn blocker() -> Blocker {
        let b = Blocker::new();
        b.load_lists([LIST.to_string()]);
        b
    }

    #[test]
    fn blocks_listed_hosts() {
        let b = blocker();
        assert!(b.should_block("https://ads.example.com/a.js", "https://site.com/", "script"));
        assert!(!b.should_block("https://site.com/a.js", "https://site.com/", "script"));
    }

    #[test]
    fn honours_exceptions_and_third_party() {
        let b = blocker();
        assert!(b.should_block("https://tracker.net/t.js", "https://site.com/", "script"));
        assert!(!b.should_block("https://tracker.net/ok.js", "https://site.com/", "script"));
        assert!(!b.should_block("https://tracker.net/t.js", "https://tracker.net/", "script"));
    }

    #[test]
    fn unloaded_blocks_nothing() {
        assert!(!Blocker::new().should_block("https://ads.example.com/", "https://a.com/", "script"));
    }

    #[test]
    fn cosmetic_rules_are_domain_scoped() {
        let b = blocker();
        let js = b.cosmetic_script("https://example.org/").unwrap();
        assert!(js.contains(".promo"));
        let js = b.cosmetic_script("https://other.com/");
        assert!(js.map_or(true, |j| !j.contains(".promo")));
    }

    #[test]
    fn content_blocking_json_is_valid() {
        let json = content_blocking_json([LIST.to_string()]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v.as_array().unwrap().len() >= 2);
    }
}
