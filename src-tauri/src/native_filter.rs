//! Sub-resource blocking. WebKitGTK only accepts declarative content-blocker rules, so the Brave
//! engine's rules are exported to WebKit's JSON format and installed on every tab.
//! (Other platforms still need their own hook: WebView2 `WebResourceRequested` on Windows,
//! `shouldInterceptRequest` on Android.)

use tauri::AppHandle;

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use std::cell::RefCell;
    use std::ffi::{c_void, CString};
    use tauri::Manager;
    use webkit2gtk::ffi;
    use webkit2gtk::glib::translate::ToGlibPtr;
    use webkit2gtk::{glib, WebViewExt};

    thread_local! {
        // Only touched on the GTK main thread.
        static FILTER: RefCell<Option<*mut ffi::WebKitUserContentFilter>> = const { RefCell::new(None) };
    }

    struct Pending {
        app: AppHandle,
        store: *mut ffi::WebKitUserContentFilterStore,
    }

    unsafe extern "C" fn saved(
        _src: *mut glib::gobject_ffi::GObject,
        res: *mut webkit2gtk::gio::ffi::GAsyncResult,
        data: *mut c_void,
    ) {
        let p = Box::from_raw(data as *mut Pending);
        let mut err: *mut glib::ffi::GError = std::ptr::null_mut();
        let filter = ffi::webkit_user_content_filter_store_save_finish(p.store, res, &mut err);
        if filter.is_null() && !err.is_null() {
            eprintln!("strata: content filter failed: {}", std::ffi::CStr::from_ptr((*err).message).to_string_lossy());
        } else {
            eprintln!("strata: content filter installed");
        }
        glib::gobject_ffi::g_object_unref(p.store as *mut _);
        if !filter.is_null() {
            FILTER.with(|f| {
                if let Some(old) = f.borrow_mut().replace(filter) {
                    ffi::webkit_user_content_filter_unref(old);
                }
            });
            super::apply_all(&p.app);
        }
    }

    unsafe extern "C" fn loaded(
        _src: *mut glib::gobject_ffi::GObject,
        res: *mut webkit2gtk::gio::ffi::GAsyncResult,
        data: *mut c_void,
    ) {
        let p = Box::from_raw(data as *mut Pending);
        let filter = ffi::webkit_user_content_filter_store_load_finish(p.store, res, std::ptr::null_mut());
        glib::gobject_ffi::g_object_unref(p.store as *mut _);
        if !filter.is_null() {
            FILTER.with(|f| {
                if f.borrow().is_none() {
                    *f.borrow_mut() = Some(filter);
                } else {
                    ffi::webkit_user_content_filter_unref(filter);
                }
            });
            super::apply_all(&p.app);
        }
    }

    /// Load the filter compiled by a previous run so blocking is active quickly at start-up.
    pub fn load_cached(app: &AppHandle, storage: &std::path::Path) {
        let (Ok(path), Ok(id)) = (CString::new(storage.to_string_lossy().as_bytes()), CString::new("strata")) else { return };
        unsafe {
            let store = ffi::webkit_user_content_filter_store_new(path.as_ptr());
            let data = Box::into_raw(Box::new(Pending { app: app.clone(), store }));
            ffi::webkit_user_content_filter_store_load(store, id.as_ptr(), std::ptr::null_mut(), Some(loaded), data as *mut c_void);
        }
    }

    /// Compile `json` into a WebKit content filter (must run on the main thread).
    pub fn install(app: &AppHandle, json: String, storage: &std::path::Path) {
        let (Ok(path), Ok(id)) = (CString::new(storage.to_string_lossy().as_bytes()), CString::new("strata")) else { return };
        unsafe {
            let store = ffi::webkit_user_content_filter_store_new(path.as_ptr());
            let bytes = glib::Bytes::from_owned(json.into_bytes());
            let data = Box::into_raw(Box::new(Pending { app: app.clone(), store }));
            ffi::webkit_user_content_filter_store_save(
                store,
                id.as_ptr(),
                bytes.to_glib_none().0,
                std::ptr::null_mut(),
                Some(saved),
                data as *mut c_void,
            );
        }
    }

    /// Attach (or detach, when ad blocking is off) the filter on one webview.
    pub fn apply(app: &AppHandle, label: &str) {
        let Some(wv) = app.get_webview(label) else { return };
        let on = app.state::<crate::AppState>().settings.lock().unwrap().adblock;
        let _ = wv.with_webview(move |pw| {
            let Some(ucm) = pw.inner().user_content_manager() else { return };
            let ucm: *mut ffi::WebKitUserContentManager = ucm.to_glib_none().0;
            unsafe {
                ffi::webkit_user_content_manager_remove_all_filters(ucm);
                if on {
                    FILTER.with(|f| {
                        if let Some(filter) = *f.borrow() {
                            ffi::webkit_user_content_manager_add_filter(ucm, filter);
                        }
                    });
                }
            }
        });
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use super::*;
    pub fn install(_: &AppHandle, _: String, _: &std::path::Path) {}
    pub fn load_cached(_: &AppHandle, _: &std::path::Path) {}
    pub fn apply(_: &AppHandle, _: &str) {}
}

pub use imp::{install, load_cached};

pub fn apply_tab(app: &AppHandle, id: u32) {
    imp::apply(app, &crate::label(id));
}

pub fn apply_all(app: &AppHandle) {
    let ids: Vec<u32> = app.state::<crate::AppState>().tabs.lock().unwrap().list.iter().map(|t| t.id).collect();
    for id in ids {
        apply_tab(app, id);
    }
}

use tauri::Manager;
