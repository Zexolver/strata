#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|a| a == "--version") {
        println!("strata {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    strata_lib::run()
}
