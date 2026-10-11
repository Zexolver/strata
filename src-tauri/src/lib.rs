mod filters;
mod native_filter;

use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use strata_core::blocker::Blocker;
use strata_core::nav::{is_allowed_navigation, resolve_input};
use strata_core::profiles::{Profile, ProfileStore, DEFAULT_ID};
use strata_core::settings::{Settings, Theme};
use tauri::webview::{PageLoadEvent, WebviewBuilder};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewUrl, WindowEvent,
};

/// Height of the tab strip plus toolbar, in logical pixels.
const CHROME_H: f64 = 88.0;
const NEWTAB: &str = "newtab.html";

struct Tab {
    id: u32,
    profile: String,
}

#[derive(Default)]
struct Tabs {
    list: Vec<Tab>,
    active: Option<u32>,
    next: u32,
    overlay: bool,
}

struct AppState {
    dir: PathBuf,
    settings: Mutex<Settings>,
    profiles: Mutex<ProfileStore>,
    tabs: Mutex<Tabs>,
    blocker: Arc<Blocker>,
}

#[derive(Serialize, Clone)]
struct TabInfo {
    id: u32,
    profile: String,
}

#[derive(Serialize, Clone)]
struct StateDto {
    settings: Settings,
    profiles: Vec<Profile>,
    tabs: Vec<TabInfo>,
    active: Option<u32>,
}

#[derive(Serialize, Clone)]
struct TabUpdate {
    id: u32,
    url: Option<String>,
    title: Option<String>,
    loading: Option<bool>,
}

fn label(id: u32) -> String {
    format!("tab-{id}")
}

fn snapshot(st: &AppState) -> StateDto {
    let tabs = st.tabs.lock().unwrap();
    StateDto {
        settings: st.settings.lock().unwrap().clone(),
        profiles: st.profiles.lock().unwrap().profiles.clone(),
        tabs: tabs.list.iter().map(|t| TabInfo { id: t.id, profile: t.profile.clone() }).collect(),
        active: tabs.active,
    }
}

fn notify(app: &AppHandle) {
    let st = app.state::<AppState>();
    let _ = app.emit_to("ui", "state", snapshot(&st));
}

/// On Linux, Tauri stacks child webviews in a vertical GTK box and ignores positions, so the
/// chrome webview is pinned to a fixed height (or given all the space when an overlay is open).
#[cfg(target_os = "linux")]
fn pin_chrome(app: &AppHandle, tall: bool) {
    use gtk::prelude::*;
    if let Some(ui) = app.get_webview("ui") {
        let _ = ui.with_webview(move |pw| {
            let w = pw.inner();
            w.set_size_request(-1, if tall { -1 } else { CHROME_H as i32 });
            if let Some(b) = w.parent().and_then(|p| p.downcast::<gtk::Box>().ok()) {
                b.set_child_packing(&w, tall, true, 0, gtk::PackType::Start);
            }
        });
    }
}

/// Position the chrome webview and the active tab; hide every other tab.
fn layout(app: &AppHandle) {
    let Some(window) = app.get_window("main") else { return };
    let (Ok(size), Ok(scale)) = (window.inner_size(), window.scale_factor()) else { return };
    let (w, h) = (size.width as f64 / scale, size.height as f64 / scale);
    let st = app.state::<AppState>();
    let tabs = st.tabs.lock().unwrap();
    #[cfg(target_os = "linux")]
    pin_chrome(app, tabs.overlay);
    if let Some(ui) = app.get_webview("ui") {
        let ui_h = if tabs.overlay { h } else { CHROME_H.min(h) };
        let _ = ui.set_position(LogicalPosition::new(0.0, 0.0));
        let _ = ui.set_size(LogicalSize::new(w, ui_h));
    }
    for t in &tabs.list {
        let Some(wv) = app.get_webview(&label(t.id)) else { continue };
        if Some(t.id) == tabs.active && !tabs.overlay {
            let _ = wv.set_position(LogicalPosition::new(0.0, CHROME_H));
            let _ = wv.set_size(LogicalSize::new(w, (h - CHROME_H).max(1.0)));
            let _ = wv.show();
        } else {
            let _ = wv.hide();
        }
    }
}

fn open_tab(app: &AppHandle, input: Option<String>, profile: Option<String>) -> Result<u32, String> {
    let st = app.state::<AppState>();
    let profiles_on = st.settings.lock().unwrap().profiles_enabled;
    let profile_id = if profiles_on { profile.unwrap_or_else(|| DEFAULT_ID.into()) } else { DEFAULT_ID.into() };
    let prof = st.profiles.lock().unwrap().get(&profile_id).cloned().ok_or("unknown profile")?;
    let id = {
        let mut t = st.tabs.lock().unwrap();
        t.next += 1;
        t.next
    };
    let url = match input.as_deref().map(resolve_input) {
        Some(u) if u != "about:blank" => WebviewUrl::External(u.parse().map_err(|_| "bad url")?),
        _ => WebviewUrl::App(NEWTAB.into()),
    };

    let (app_nav, app_load, app_title) = (app.clone(), app.clone(), app.clone());
    let (blocker_nav, blocker_load) = (st.blocker.clone(), st.blocker.clone());
    let adblock_nav = app.clone();
    let mut builder = WebviewBuilder::new(label(id), url)
        .data_directory(ProfileStore::data_dir(&st.dir, &prof.id))
        .on_navigation(move |u| {
            if u.scheme() == "tauri" || u.host_str() == Some("tauri.localhost") {
                return true;
            }
            if !is_allowed_navigation(u) {
                return false;
            }
            let on = adblock_nav.state::<AppState>().settings.lock().unwrap().adblock;
            if on && blocker_nav.should_block(u.as_str(), u.as_str(), "document") {
                return false;
            }
            let _ = app_nav.emit_to("ui", "tab-update", TabUpdate { id, url: Some(u.to_string()), title: None, loading: Some(true) });
            true
        })
        .on_page_load(move |wv, payload| {
            let url = payload.url().to_string();
            let loading = payload.event() == PageLoadEvent::Started;
            let on = app_load.state::<AppState>().settings.lock().unwrap().adblock;
            if on && loading {
                if let Some(js) = blocker_load.cosmetic_script(&url) {
                    let _ = wv.eval(js);
                }
            }
            let _ = app_load.emit_to("ui", "tab-update", TabUpdate { id, url: Some(url), title: None, loading: Some(loading) });
        })
        .on_document_title_changed(move |_, title| {
            let _ = app_title.emit_to("ui", "tab-update", TabUpdate { id, url: None, title: Some(title), loading: None });
        })
        .on_new_window({
            let app = app.clone();
            let profile = prof.id.clone();
            move |url, _| {
                if is_allowed_navigation(&url) {
                    let _ = open_tab(&app, Some(url.to_string()), Some(profile.clone()));
                }
                tauri::webview::NewWindowResponse::Deny
            }
        });
    if let Some(p) = prof.proxy.as_deref().and_then(|p| p.parse().ok()) {
        builder = builder.proxy_url(p);
    }

    let window = app.get_window("main").ok_or("no window")?;
    window
        .add_child(builder, LogicalPosition::new(0.0, CHROME_H), LogicalSize::new(800.0, 600.0))
        .map_err(|e| e.to_string())?;
    {
        let mut t = st.tabs.lock().unwrap();
        t.list.push(Tab { id, profile: prof.id });
        t.active = Some(id);
        t.overlay = false;
    }
    layout(app);
    native_filter::apply_tab(app, id);
    notify(app);
    Ok(id)
}

fn tab_webview(app: &AppHandle, id: u32) -> Result<tauri::Webview, String> {
    app.get_webview(&label(id)).ok_or_else(|| "no such tab".to_string())
}

#[tauri::command]
fn get_state(st: State<AppState>) -> StateDto {
    snapshot(&st)
}

#[tauri::command]
fn new_tab(app: AppHandle, url: Option<String>, profile: Option<String>) -> Result<u32, String> {
    open_tab(&app, url, profile)
}

#[tauri::command]
fn close_tab(app: AppHandle, st: State<AppState>, id: u32) -> Result<(), String> {
    if let Ok(wv) = tab_webview(&app, id) {
        let _ = wv.close();
    }
    let empty = {
        let mut t = st.tabs.lock().unwrap();
        let idx = t.list.iter().position(|x| x.id == id);
        t.list.retain(|x| x.id != id);
        if t.active == Some(id) {
            let next = idx.map(|i| i.min(t.list.len().saturating_sub(1)));
            t.active = next.and_then(|i| t.list.get(i)).map(|x| x.id);
        }
        t.list.is_empty()
    };
    if empty {
        open_tab(&app, None, None)?;
    } else {
        layout(&app);
        notify(&app);
    }
    Ok(())
}

#[tauri::command]
fn switch_tab(app: AppHandle, st: State<AppState>, id: u32) {
    {
        let mut t = st.tabs.lock().unwrap();
        if t.list.iter().any(|x| x.id == id) {
            t.active = Some(id);
            t.overlay = false;
        }
    }
    layout(&app);
    notify(&app);
}

#[tauri::command]
fn navigate(app: AppHandle, id: u32, input: String) -> Result<(), String> {
    let url = resolve_input(&input);
    let wv = tab_webview(&app, id)?;
    let target: tauri::Url = if url == "about:blank" {
        return Ok(());
    } else {
        url.parse().map_err(|_| "bad url")?
    };
    wv.navigate(target).map_err(|e| e.to_string())
}

#[tauri::command]
fn history_back(app: AppHandle, id: u32) -> Result<(), String> {
    tab_webview(&app, id)?.eval("history.back()").map_err(|e| e.to_string())
}

#[tauri::command]
fn history_forward(app: AppHandle, id: u32) -> Result<(), String> {
    tab_webview(&app, id)?.eval("history.forward()").map_err(|e| e.to_string())
}

#[tauri::command]
fn reload(app: AppHandle, id: u32) -> Result<(), String> {
    tab_webview(&app, id)?.reload().map_err(|e| e.to_string())
}

fn save_settings(st: &AppState) {
    let _ = st.settings.lock().unwrap().save(&st.dir.join("settings.json"));
}

#[tauri::command]
fn set_theme(app: AppHandle, st: State<AppState>, theme: Theme) {
    st.settings.lock().unwrap().theme = theme;
    save_settings(&st);
    notify(&app);
}

#[tauri::command]
fn set_adblock(app: AppHandle, st: State<AppState>, on: bool) {
    st.settings.lock().unwrap().adblock = on;
    save_settings(&st);
    native_filter::apply_all(&app);
    notify(&app);
}

#[tauri::command]
fn set_profiles_enabled(app: AppHandle, st: State<AppState>, on: bool) {
    st.settings.lock().unwrap().profiles_enabled = on;
    save_settings(&st);
    notify(&app);
}

#[tauri::command]
fn add_profile(app: AppHandle, st: State<AppState>, name: String, color: String, proxy: Option<String>) -> Result<Profile, String> {
    let mut p = st.profiles.lock().unwrap();
    let prof = p.add(&name, &color, proxy).map_err(|e| e.to_string())?;
    let _ = p.save(&st.dir.join("profiles.json"));
    drop(p);
    notify(&app);
    Ok(prof)
}

#[tauri::command]
fn remove_profile(app: AppHandle, st: State<AppState>, id: String) -> Result<(), String> {
    let in_use: Vec<u32> = st.tabs.lock().unwrap().list.iter().filter(|t| t.profile == id).map(|t| t.id).collect();
    for tab in in_use {
        close_tab(app.clone(), st.clone(), tab)?;
    }
    {
        let mut p = st.profiles.lock().unwrap();
        p.remove(&id).map_err(|e| e.to_string())?;
        let _ = p.save(&st.dir.join("profiles.json"));
    }
    let _ = std::fs::remove_dir_all(st.dir.join("profiles").join(&id));
    notify(&app);
    Ok(())
}

#[tauri::command]
fn set_overlay(app: AppHandle, st: State<AppState>, on: bool) {
    st.tabs.lock().unwrap().overlay = on;
    layout(&app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_state, new_tab, close_tab, switch_tab, navigate, history_back, history_forward,
            reload, set_theme, set_adblock, set_profiles_enabled, add_profile, remove_profile,
            set_overlay
        ])
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let blocker = Arc::new(Blocker::new());
            native_filter::load_cached(app.handle(), &dir.join("filters").join("webkit-store"));
            filters::spawn_updater(app.handle().clone(), blocker.clone(), dir.join("filters"));
            app.manage(AppState {
                settings: Mutex::new(Settings::load(&dir.join("settings.json"))),
                profiles: Mutex::new(ProfileStore::load(&dir.join("profiles.json"))),
                tabs: Mutex::new(Tabs::default()),
                dir,
                blocker,
            });

            let window = tauri::window::WindowBuilder::new(app, "main")
                .title("Strata")
                .inner_size(1200.0, 800.0)
                .min_inner_size(360.0, 300.0)
                .build()?;
            window.add_child(
                WebviewBuilder::new("ui", WebviewUrl::App("index.html".into())),
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(1200.0, CHROME_H),
            )?;
            let handle = app.handle().clone();
            window.on_window_event(move |e| {
                if matches!(e, WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }) {
                    layout(&handle);
                }
            });
            // `strata [--profile ID] URL...`: open the given pages (profile applies to the URLs after it).
            let mut profile: Option<String> = None;
            let mut opened = false;
            let mut args = std::env::args().skip(1);
            while let Some(a) = args.next() {
                if a == "--profile" {
                    profile = args.next();
                } else {
                    open_tab(app.handle(), Some(a), profile.clone())?;
                    opened = true;
                }
            }
            if !opened {
                open_tab(app.handle(), None, None)?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Strata");
}
