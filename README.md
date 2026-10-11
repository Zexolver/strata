# Strata

A lightweight, secure browser built on Tauri v2 (system webview, no bundled engine).

- **Ad blocking**: Brave's `adblock` Rust engine (EasyList, EasyPrivacy, uBlock, Brave unbreak lists, refreshed every 3 days).
  Linux: rules exported to WebKitGTK content filters (sub-resources) + engine-driven navigation and cosmetic filtering.
- **Isolated profiles (experimental, off by default)**: Settings > Experimental. Each profile has its own
  cookies/storage/cache in its own engine data directory (and optionally its own proxy), so sites in one profile cannot see another.
- Theme: system / light / dark.
- CLI: `strata [--profile ID] URL...`

## Status
- Linux (WebKitGTK 4.1 from the distro, no bundled libraries): working, verified (isolation + ad blocking).
- Windows: type-checks; sub-resource blocking (WebView2 `WebResourceRequested`) and runtime testing still TODO.
- Android/iOS: not done. Tauri child webviews are desktop-only, so mobile needs a different tab design.
- Not yet: generic cosmetic rules, downloads UI, bookmarks/history, find-in-page.

## Build (Linux)
    sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev
    cargo build --release -p strata
Targets: x86_64, x86, aarch64, armv7 (CI in `.github/workflows/build.yml`).
