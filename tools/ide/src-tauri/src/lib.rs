// lazy static initialization

use toml::Table;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn parse_toml(toml_str: &str) -> String {
    let val = toml_str.parse::<Table>().unwrap();
    format!("this is your val: {}", val["foo"].as_str().unwrap())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // storage
    // logger init

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet, parse_toml])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
