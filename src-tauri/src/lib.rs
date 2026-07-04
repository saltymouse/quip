pub mod import;
pub mod scan;
pub mod thumbs;
pub mod volumes;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            volumes::start_watcher(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            volumes::list_cards,
            volumes::eject_volume,
            scan::scan_card,
            thumbs::make_thumb,
            thumbs::make_preview,
            import::check_destination,
            import::render_pattern_preview,
            import::import_sessions,
            import::cancel_import,
            import::delete_from_card,
            import::reveal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
