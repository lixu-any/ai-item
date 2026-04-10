use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use tauri::{AppHandle, Emitter, State};
use base64::{Engine as _, engine::general_purpose};

pub enum PtyControlMsg {
    Data(Vec<u8>),
    Resize { cols: u16, rows: u16 },
    Close,
}

pub struct PtySession {
    pub tx: mpsc::SyncSender<PtyControlMsg>,
}

#[derive(Default)]
pub struct PtySessionPool(pub Arc<Mutex<HashMap<String, PtySession>>>);

#[tauri::command]
pub async fn open_pty_session(
    app_handle: AppHandle,
    state: State<'_, PtySessionPool>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    // 跨平台 shell 检测：Windows 用 cmd.exe，Unix 用 $SHELL
    #[cfg(target_os = "windows")]
    let shell = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
    #[cfg(not(target_os = "windows"))]
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    let mut cmd = CommandBuilder::new(&shell);
    
    // 设置环境变量，增强终端兼容性
    cmd.env("TERM", "xterm-256color");
    
    let mut child = pty_pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    let (tx, rx) = mpsc::sync_channel::<PtyControlMsg>(2048);

    // 保存到池中
    {
        let mut pool = state.0.lock().unwrap();
        pool.insert(session_id.clone(), PtySession { tx });
    }

    let app_handle_clone = app_handle.clone();
    let session_id_for_reader = session_id.clone();
    let session_id_for_control = session_id.clone();
    
    // 读取 PTY 输出并发送到前端
    let mut reader = pty_pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let b64_data = general_purpose::STANDARD.encode(&buffer[..n]);
                    let _ = app_handle_clone.emit(&format!("sse-data-{}", session_id_for_reader), b64_data);
                }
                Err(_) => break,
            }
        }
        println!("PTY reader thread exited for session {}", session_id_for_reader);
    });

    // 处理控制信号（写入数据和重置尺寸）
    let master = pty_pair.master;
    let mut writer = master.take_writer().map_err(|e| e.to_string())?;
    thread::spawn(move || {
        while let Ok(msg) = rx.recv() {
            match msg {
                PtyControlMsg::Data(data) => {
                    if let Err(e) = writer.write_all(&data) {
                        println!("PTY write error: {}", e);
                        break;
                    }
                    let _ = writer.flush();
                }
                PtyControlMsg::Resize { cols, rows } => {
                    let _ = master.resize(PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
                PtyControlMsg::Close => {
                    println!("Worker: 收到 Close 请求，正在结束 PTY 会话...");
                    break;
                }
            }
        }
        // 尝试杀掉子进程
        let _ = child.kill();
        println!("PTY control thread exited for session {}", session_id_for_control);
    });

    Ok(())
}

#[tauri::command]
pub async fn write_to_pty(
    state: State<'_, PtySessionPool>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    let pool = state.0.lock().unwrap();
    if let Some(session) = pool.get(&session_id) {
        let bytes = general_purpose::STANDARD.decode(data.as_bytes()).unwrap_or_default();
        let _ = session.tx.send(PtyControlMsg::Data(bytes));
        Ok(())
    } else {
        Err("Session not found".to_string())
    }
}

#[tauri::command]
pub async fn resize_pty_session(
    state: State<'_, PtySessionPool>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let pool = state.0.lock().unwrap();
    if let Some(session) = pool.get(&session_id) {
        let _ = session.tx.send(PtyControlMsg::Resize { cols, rows });
        Ok(())
    } else {
        Err("Session not found".to_string())
    }
}

#[tauri::command]
pub async fn close_pty_session(
    state: State<'_, PtySessionPool>,
    session_id: String,
) -> Result<(), String> {
    let mut pool = state.0.lock().unwrap();
    if let Some(sess) = pool.remove(&session_id) {
        let _ = sess.tx.send(PtyControlMsg::Close);
    }
    Ok(())
}
