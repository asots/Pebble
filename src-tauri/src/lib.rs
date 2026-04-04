mod commands;
mod events;
mod state;

use state::AppState;
use std::path::PathBuf;
use tauri::Manager;

fn get_db_path(app: &tauri::App) -> PathBuf {
    let app_data = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data directory");
    std::fs::create_dir_all(&app_data).expect("Failed to create app data directory");
    let db_dir = app_data.join("db");
    std::fs::create_dir_all(&db_dir).expect("Failed to create db directory");
    db_dir.join("pebble.db")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pebble=debug,pebble_store=debug".into()),
        )
        .init();

    tauri::Builder::default()
        .setup(|app| {
            let db_path = get_db_path(app);
            tracing::info!("Database path: {}", db_path.display());
            let store =
                pebble_store::Store::open(&db_path).expect("Failed to open database");
            tracing::info!("Database initialized successfully");
            app.manage(AppState::new(store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::health::health_check,])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
