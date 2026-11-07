pub mod converter;
pub mod exporter;
pub mod parser;

#[cfg(feature = "desktop")]
mod commands;

#[cfg(feature = "desktop")]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::convert_file,
            commands::reveal_output
        ])
        .run(tauri::generate_context!())
        .expect("No se pudo iniciar XD2Figma");
}
