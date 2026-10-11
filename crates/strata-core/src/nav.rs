use url::Url;

pub const DEFAULT_SEARCH: &str = "https://duckduckgo.com/?q=";

/// Turn whatever was typed in the address bar into a URL to load.
pub fn resolve_input(input: &str) -> String {
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
    format!("{DEFAULT_SEARCH}{}", url::form_urlencoded::byte_serialize(s.as_bytes()).collect::<String>())
}

/// Only web pages may be opened by the browser; blocks `javascript:`, `data:`, `file:` from links etc.
pub fn is_allowed_navigation(url: &Url) -> bool {
    match url.scheme() {
        "http" | "https" => true,
        "about" => url.as_str() == "about:blank",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_hosts_and_searches() {
        assert_eq!(resolve_input("https://a.com/x"), "https://a.com/x");
        assert_eq!(resolve_input("example.com"), "https://example.com/");
        assert_eq!(resolve_input("localhost:8080"), "http://localhost:8080/");
        assert_eq!(resolve_input("rust lang"), "https://duckduckgo.com/?q=rust+lang");
        assert_eq!(resolve_input("javascript:alert(1)"), "https://duckduckgo.com/?q=javascript%3Aalert%281%29");
        assert_eq!(resolve_input(""), "about:blank");
    }

    #[test]
    fn navigation_policy() {
        assert!(is_allowed_navigation(&Url::parse("https://a.com").unwrap()));
        assert!(!is_allowed_navigation(&Url::parse("javascript:1").unwrap()));
        assert!(!is_allowed_navigation(&Url::parse("file:///etc/passwd").unwrap()));
    }
}
