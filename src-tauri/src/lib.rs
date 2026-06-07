mod dns;
mod monitor;
mod storage;
mod types;

use chrono::Utc;
use std::sync::{Arc, Mutex};
use tauri::State;
use types::{InterfaceSample, NetworkAdapter};

pub struct AppState {
    pub storage: Arc<Mutex<storage::Storage>>,
}

#[tauri::command]
fn get_adapters() -> Vec<NetworkAdapter> {
    dns::list_adapters()
}

#[tauri::command]
fn enable_dns(adapter: String) -> Result<(), String> {
    dns::enable(&adapter)
}

#[tauri::command]
fn disable_dns(adapter: String) -> Result<(), String> {
    dns::disable(&adapter)
}

#[tauri::command]
fn query_usage(
    interface: Option<String>,
    since_hours: i64,
    state: State<AppState>,
) -> Vec<InterfaceSample> {
    let since = Utc::now() - chrono::Duration::hours(since_hours.max(1));
    state
        .storage
        .lock()
        .unwrap()
        .query_samples(interface.as_deref(), since)
        .unwrap_or_default()
}

#[tauri::command]
fn prune_data(keep_days: i64, state: State<AppState>) -> usize {
    state
        .storage
        .lock()
        .unwrap()
        .prune(keep_days.max(1))
        .unwrap_or(0)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_adapters,
            enable_dns,
            disable_dns,
            query_usage,
            prune_data,
        ])
        .setup(|app| {
            let storage = Arc::new(Mutex::new(
                storage::Storage::new().expect("storage init failed"),
            ));

            app.manage(AppState {
                storage: Arc::clone(&storage),
            });

            monitor::start(app.handle().clone(), storage);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running application");
}
