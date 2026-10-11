fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "get_state", "new_tab", "close_tab", "switch_tab", "navigate", "history_back",
            "history_forward", "reload", "set_theme", "set_adblock", "set_profiles_enabled",
            "add_profile", "remove_profile", "set_overlay",
        ])),
    )
    .expect("tauri build failed");
}
