// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

mod db;
mod ssh;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ssh::SessionPool::default())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            db::init_db(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            db::add_host,
            db::update_host,
            db::get_hosts,
            db::delete_host,
            db::add_group,
            db::get_groups,
            ssh::open_ssh_session,
            ssh::write_to_ssh,
            ssh::resize_ssh_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
