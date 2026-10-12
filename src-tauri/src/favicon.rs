//! Tab icons. On Linux the icon WebKit already fetched for the page (inside the tab's own
//! profile/proxy) is read locally and sent to the chrome as a PNG data URL, so no extra
//! requests are made on the user's behalf.

use tauri::AppHandle;

#[cfg(target_os = "linux")]
pub fn watch(app: &AppHandle, id: u32) {
    use gtk::cairo::{Format, ImageSurface};
    use tauri::{Emitter, Manager};
    use webkit2gtk::{WebContextExt, WebViewExt};
    let Some(wv) = app.get_webview(&crate::label(id)) else { return };
    let handle = app.clone();
    let st = app.state::<crate::AppState>();
    let profile = st.tabs.lock().unwrap().list.iter().find(|t| t.id == id).map(|t| t.profile.clone()).unwrap_or_default();
    let dir = st.dir.join("profiles").join(profile).join("favicons");
    let _ = wv.with_webview(move |pw| {
        if let Some(ctx) = pw.inner().context() {
            ctx.set_favicon_database_directory(dir.to_str());
        }
        pw.inner().connect_favicon_notify(move |w| {
            let Some(surface) = w.favicon() else { return };
            let Ok(img) = ImageSurface::try_from(surface) else { return };
            if img.format() == Format::Invalid || img.width() > 256 {
                return;
            }
            let mut png = Vec::new();
            if img.write_to_png(&mut png).is_err() {
                return;
            }
            let icon = format!("data:image/png;base64,{}", b64(&png));
            let _ = handle.emit_to(
                "ui",
                "tab-update",
                crate::TabUpdate { id, url: None, title: None, loading: None, icon: Some(icon) },
            );
        });
    });
}

#[cfg(not(target_os = "linux"))]
pub fn watch(_: &AppHandle, _: u32) {}

#[cfg(target_os = "linux")]
fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}
