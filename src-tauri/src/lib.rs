use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

mod db;
mod ssh;
mod pty;
mod ai;
mod fonts;
mod completions;
mod sftp;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ssh::SessionPool(Default::default()))
        .manage(pty::PtySessionPool(Default::default()))
        .manage(sftp::SftpPool(Default::default()))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
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
                
                // 设置完尺寸后居中显示，避免位置偏移
                println!("Backend: Centering and showing main window.");
                let _ = main_window.center();
                let _ = main_window.show();
            } else {
                println!("Backend: Main window not found by label 'main'!");
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            db::get_hosts,
            db::add_host,
            db::update_host,
            db::delete_host,
            db::get_groups,
            db::add_group,
            db::update_group,
            db::delete_group,
            db::save_setting,
            db::get_setting,
            ssh::open_ssh_session,
            ssh::write_to_ssh,
            ssh::resize_ssh_session,
            ssh::close_ssh_session,
            pty::open_pty_session,
            pty::write_to_pty,
            pty::resize_pty_session,
            pty::close_pty_session,
            db::add_snippet,
            db::get_snippets,
            db::delete_snippet,
            db::update_snippet,
            db::add_credential,
            db::get_credentials,
            db::delete_credential,
            db::update_credential,
            ai::ask_ai,
            db::export_data,
            db::import_data,
            db::record_recent,
            db::get_recents,
            db::clear_recents,
            db::log_terminal_command,
            db::get_recent_session_logs,
            fonts::get_system_fonts,
            ssh::get_host_stats,
            completions::get_completions,
            sftp::sftp_connect,
            sftp::sftp_read_dir,
            sftp::sftp_close,
            sftp::sftp_mkdir,
            sftp::sftp_delete,
            sftp::sftp_rename,
            sftp::sftp_download_file,
            sftp::sftp_upload_file,
            sftp::sftp_compress,
            sftp::sftp_extract,
            sftp::sftp_copy,
            sftp::sftp_exec,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
