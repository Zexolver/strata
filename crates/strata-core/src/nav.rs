use url::Url;

pub struct SearchEngine {
    pub id: &'static str,
    pub name: &'static str,
    /// Query template; `%s` is replaced by the URL-encoded search terms.
    pub template: &'static str,
    pub home: &'static str,
}

pub const ENGINES: &[SearchEngine] = &[
    SearchEngine { id: "duckduckgo", name: "DuckDuckGo", template: "https://duckduckgo.com/?q=%s", home: "https://duckduckgo.com/" },
    SearchEngine { id: "brave", name: "Brave Search", template: "https://search.brave.com/search?q=%s", home: "https://search.brave.com/" },
    SearchEngine { id: "startpage", name: "Startpage", template: "https://www.startpage.com/do/search?q=%s", home: "https://www.startpage.com/" },
    SearchEngine { id: "mojeek", name: "Mojeek", template: "https://www.mojeek.com/search?q=%s", home: "https://www.mojeek.com/" },
    SearchEngine { id: "ecosia", name: "Ecosia", template: "https://www.ecosia.org/search?q=%s", home: "https://www.ecosia.org/" },
    SearchEngine { id: "google", name: "Google", template: "https://www.google.com/search?q=%s", home: "https://www.google.com/" },
    SearchEngine { id: "bing", name: "Bing", template: "https://www.bing.com/search?q=%s", home: "https://www.bing.com/" },
];

pub const DEFAULT_ENGINE: &str = "duckduckgo";

pub fn engine(id: &str) -> Option<&'static SearchEngine> {
    ENGINES.iter().find(|e| e.id == id)
}

/// Is `template` a usable custom search URL (http(s) with a `%s` placeholder)?
pub fn valid_template(template: &str) -> bool {
    template.contains("%s")
        && Url::parse(&template.replace("%s", "x")).map_or(false, |u| matches!(u.scheme(), "http" | "https") && u.host().is_some())
}

/// Turn whatever was typed in the address bar into a URL to load; plain text is searched with `template`.
pub fn resolve_input(input: &str, template: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return "about:blank".into();
    }
    if let Ok(u) = Url::parse(s) {
        if matches!(u.scheme(), "http" | "https" | "about" | "file") {
            return u.into();
        }
    }
    let looks_host = !s.contains(char::is_whitespace)
        && (s.contains('.') || s.starts_with("localhost") || s.parse::<std::net::IpAddr>().is_ok())
        && !s.starts_with('.')
        && !s.ends_with('.');
    if looks_host {
        let scheme = if s.starts_with("localhost") || s.starts_with("127.") { "http" } else { "https" };
        if let Ok(u) = Url::parse(&format!("{scheme}://{s}")) {
            return u.into();
        }
    }
    template.replace("%s", &url::form_urlencoded::byte_serialize(s.as_bytes()).collect::<String>())
}

/// Only web pages may be opened by the browser; blocks `javascript:`, `data:`, `file:` from links etc.
pub fn is_allowed_navigation(url: &Url) -> bool {
    match url.scheme() {
        "http" | "https" => true,
        "about" => matches!(url.as_str(), "about:blank" | "about:srcdoc"),
        "blob" => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_hosts_and_searches() {
        let t = engine(DEFAULT_ENGINE).unwrap().template;
        assert_eq!(resolve_input("https://a.com/x", t), "https://a.com/x");
        assert_eq!(resolve_input("example.com", t), "https://example.com/");
        assert_eq!(resolve_input("localhost:8080", t), "http://localhost:8080/");
        assert_eq!(resolve_input("rust lang", t), "https://duckduckgo.com/?q=rust+lang");
        assert_eq!(resolve_input("javascript:alert(1)", t), "https://duckduckgo.com/?q=javascript%3Aalert%281%29");
        assert_eq!(resolve_input("", t), "about:blank");
    }

    #[test]
    fn templates() {
        assert!(valid_template("https://x.org/s?q=%s"));
        assert!(!valid_template("https://x.org/s"));
        assert!(!valid_template("javascript:%s"));
        assert!(ENGINES.iter().all(|e| valid_template(e.template)));
    }

    #[test]
    fn navigation_policy() {
        assert!(is_allowed_navigation(&Url::parse("https://a.com").unwrap()));
        assert!(!is_allowed_navigation(&Url::parse("javascript:1").unwrap()));
        assert!(!is_allowed_navigation(&Url::parse("file:///etc/passwd").unwrap()));
        assert!(is_allowed_navigation(&Url::parse("about:srcdoc").unwrap()));
    }
}
