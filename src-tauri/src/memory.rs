//! Keeps RAM low: WebKit cache limits, and unloading background tabs that have been idle.

use crate::{label, notify, AppState};
use std::time::Duration;
use tauri::{AppHandle, Manager};

/// Process-wide WebKit memory limits (must run before any web view exists).
pub fn init() {
    #[cfg(target_os = "linux")]
    {
        use webkit2gtk::{MemoryPressureSettings, WebsiteDataManager};
        let mut m = MemoryPressureSettings::new();
        m.set_memory_limit(256); // MB per web process before WebKit starts dropping caches
        m.set_conservative_threshold(0.4);
        m.set_strict_threshold(0.6);
        m.set_poll_interval(5.0);
        WebsiteDataManager::set_memory_pressure_settings(&mut m);
    }
}

/// Per-tab tuning: smallest cache model, no back/forward page cache.
pub fn tune(app: &AppHandle, id: u32) {
    #[cfg(target_os = "linux")]
    if let Some(wv) = app.get_webview(&label(id)) {
        let _ = wv.with_webview(|pw| {
            use webkit2gtk::{CacheModel, SettingsExt, WebContextExt, WebViewExt};
            let v = pw.inner();
            if let Some(ctx) = v.context() {
                ctx.set_cache_model(CacheModel::DocumentViewer);
            }
            if let Some(s) = WebViewExt::settings(&v) {
                s.set_enable_page_cache(false);
            }
        });
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (app, id);
}

/// Every 30 s, unload tabs idle longer than the `discard_minutes` setting. The tab stays in the
/// strip and reloads from its URL when selected.
pub fn spawn_reaper(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(30));
        let st = app.state::<AppState>();
        let minutes = st.settings.lock().unwrap().discard_minutes;
        if minutes == 0 {
            continue;
        }
        let ids: Vec<u32> = {
            let mut t = st.tabs.lock().unwrap();
            let (active, settings) = (t.active, t.settings);
            let limit = Duration::from_secs(minutes as u64 * 60);
            t.list
                .iter_mut()
                .filter(|x| Some(x.id) != active && Some(x.id) != settings && !x.discarded && x.url.is_some() && x.last_active.elapsed() > limit)
                .map(|x| {
                    x.discarded = true;
                    x.id
                })
                .collect()
        };
        if ids.is_empty() {
            continue;
        }
        let app2 = app.clone();
        let _ = app.run_on_main_thread(move || {
            for id in ids {
                if let Some(wv) = app2.get_webview(&label(id)) {
                    let _ = wv.close();
                }
            }
            notify(&app2);
        });
    });
}
