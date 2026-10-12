fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "get_state", "open_settings", "new_tab", "close_tab", "switch_tab", "navigate", "history_back",
            "history_forward", "reload", "update_settings",
            "add_profile", "remove_profile", "set_overlay",
        ])),
    )
    .expect("tauri build failed");
}
