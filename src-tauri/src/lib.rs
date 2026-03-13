use tauri::Manager;

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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            db::init_db(app.handle())?;
            
            // 恢复窗口尺寸
            if let Some(main_window) = app.get_webview_window("main") {
                println!("Backend: Found main window, attempting to restore size...");
                let width = db::get_setting_internal(app.handle(), "window_width")
                    .ok()
                    .flatten()
                    .and_then(|v| {
                        println!("Backend: Read width string: {:?}", v);
                        v.parse::<u32>().ok()
                    });
                let height = db::get_setting_internal(app.handle(), "window_height")
                    .ok()
                    .flatten()
                    .and_then(|v| {
                        println!("Backend: Read height string: {:?}", v);
                        v.parse::<u32>().ok()
                    });
                
                if let (Some(w), Some(h)) = (width, height) {
                    println!("Backend: Applying PhysicalSize: {}x{}", w, h);
                    let _ = main_window.set_size(tauri::Size::Physical(tauri::PhysicalSize { width: w, height: h }));
                } else {
                    println!("Backend: No saved size found or parse failed.");
                }
                
                // 设置完尺寸后再显示窗口，避免闪烁
                println!("Backend: Showing main window.");
                let _ = main_window.show();
            } else {
                println!("Backend: Main window not found by label 'main'!");
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            db::add_host,
            db::get_host,
            db::update_host,
            db::get_hosts,
            db::delete_host,
            db::add_group,
            db::get_group,
            db::update_group,
            db::delete_group,
            db::get_groups,
            db::save_setting,
            db::get_setting,
            ssh::open_ssh_session,
            ssh::write_to_ssh,
            ssh::resize_ssh_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
