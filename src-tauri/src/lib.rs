/// lib.rs — Tauri 插件入口，暴露所有命令

mod commands;
mod desensitize;
mod docx_parser;
mod excel;
mod llm;
mod pdf_parser;
mod txt;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            open_excel,
            scan_sensitive,
            desensitize_file,
            chat_with_llm,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
