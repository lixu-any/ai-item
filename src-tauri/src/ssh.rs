use ssh2::{Session, ErrorCode};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

pub enum SshControlMsg {
    Data(Vec<u8>),
    Resize { cols: u32, rows: u32 },
}

pub struct SshSession {
    pub tx: mpsc::Sender<SshControlMsg>,
}

#[derive(Default)]
pub struct SessionPool(pub Arc<Mutex<HashMap<String, SshSession>>>);

// libssh2 error code for EAGAIN
const EAGAIN: i32 = -37;

#[tauri::command]
pub async fn open_ssh_session(
    app_handle: AppHandle,
    state: State<'_, SessionPool>,
    session_id: String,
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    private_key: Option<String>,
) -> Result<(), String> {
    let tcp = TcpStream::connect(format!("{}:{}", host, port)).map_err(|e| e.to_string())?;
    tcp.set_nonblocking(true).map_err(|e| e.to_string())?;
    
    let mut sess = Session::new().map_err(|e| e.to_string())?;
    sess.set_tcp_stream(tcp);
    sess.set_blocking(false);
    
    // Handshake
    loop {
        match sess.handshake() {
            Ok(_) => break,
            Err(e) if e.code() == ErrorCode::Session(EAGAIN) => {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(e) => return Err(format!("Handshake failed: {}", e)),
        }
    }

    // Authenticate
    if let Some(pkey_raw) = private_key {
        let pkey = if pkey_raw.trim().starts_with("-----BEGIN") {
            pkey_raw
        } else {
            // 视为路径，处理 ~ 符号
            let path_str = pkey_raw.replace("~", &std::env::var("HOME").unwrap_or_default());
            std::fs::read_to_string(path_str).map_err(|e| format!("读取私钥文件失败: {}", e))?
        };

        // 使用私钥认证
        loop {
            match sess.userauth_pubkey_memory(&username, None, &pkey, password.as_deref()) {
                Ok(_) => break,
                Err(e) if e.code() == ErrorCode::Session(EAGAIN) => {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(e) => return Err(format!("Private key auth failed: {}", e)),
            }
        }
    } else if let Some(pwd) = password {
        // 使用密码认证
        loop {
            match sess.userauth_password(&username, &pwd) {
                Ok(_) => break,
                Err(e) if e.code() == ErrorCode::Session(EAGAIN) => {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(e) => return Err(format!("Password auth failed: {}", e)),
            }
        }
    } else {
        return Err("Password or Private Key required".to_string());
    }

    // Open Channel
    let mut channel = loop {
        match sess.channel_session() {
            Ok(c) => break c,
            Err(e) if e.code() == ErrorCode::Session(EAGAIN) => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("Channel failed: {}", e)),
        }
    };

    // Request PTY
    loop {
        match channel.request_pty("xterm", None, None) {
            Ok(_) => break,
            Err(e) if e.code() == ErrorCode::Session(EAGAIN) => {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(e) => return Err(format!("PTY failed: {}", e)),
        }
    }

    // Start Shell
    loop {
        match channel.shell() {
            Ok(_) => break,
            Err(e) if e.code() == ErrorCode::Session(EAGAIN) => {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(e) => return Err(format!("Shell failed: {}", e)),
        }
    }

    let (tx, rx) = mpsc::channel::<SshControlMsg>();
    
    let session_id_clone = session_id.clone();
    let app_handle_clone = app_handle.clone();
    
    thread::spawn(move || {
        println!("Worker: 开始为 Session {} 启动循环...", session_id_clone);
        let mut buffer = [0u8; 8192];
        loop {
            let mut active = false;

            // Read from SSH
            match channel.read(&mut buffer) {
                Ok(0) => {
                    println!("Worker: SSH channel read 遇到 EOF (Ok(0))");
                    break;
                }
                Ok(n) => {
                    active = true;
                    let data = &buffer[..n];
                    println!("Worker: 从 SSH 读取到了 {} bytes，准备通过事件发送", n);
                    if let Err(e) = app_handle_clone.emit(
                        &format!("sse-data-{}", session_id_clone),
                        data.to_vec(),
                    ) {
                        println!("Worker: emit 发送事件失败: {}", e);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Normal in non-blocking
                }
                Err(e) => {
                    println!("Worker: SSH channel 发生致命读取错误: {:?}", e);
                    break;
                }
            }

            // Process Control Messages (Write to SSH or Resize)
            while let Ok(msg) = rx.try_recv() {
                active = true;
                match msg {
                    SshControlMsg::Data(data) => {
                        let mut written = 0;
                        while written < data.len() {
                            match channel.write(&data[written..]) {
                                Ok(n) => {
                                    written += n;
                                }
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    thread::sleep(Duration::from_millis(10));
                                    continue;
                                }
                                Err(e) => {
                                    println!("Worker: 向 SSH channel 写入失败: {:?}", e);
                                    break;
                                }
                            }
                        }
                        let _ = channel.flush();
                    }
                    SshControlMsg::Resize { cols, rows } => {
                        println!("Worker: 收到 Resize 请求, cols: {}, rows: {}", cols, rows);
                        loop {
                            match channel.request_pty_size(cols, rows, None, None) {
                                Ok(_) => {
                                    println!("Worker: PTY Size 同步成功");
                                    break;
                                }
                                Err(e) if e.code() == ErrorCode::Session(EAGAIN) => {
                                    thread::sleep(Duration::from_millis(50));
                                    continue;
                                }
                                Err(e) => {
                                    println!("Worker: PTY Size 同步失败: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if !active {
                // To avoid 100% CPU usage
                thread::sleep(Duration::from_millis(20));
            }
        }
        println!("Worker: SSH 会话 {} 线程已退出循环结束工作", session_id_clone);
    });

    state.0.lock().unwrap().insert(
        session_id.clone(),
        SshSession { tx },
    );

    Ok(())
}

#[tauri::command]
pub async fn write_to_ssh(
    state: State<'_, SessionPool>,
    session_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    println!("收到前端输入请求，SessionID: {}, 数据长度: {}", session_id, data.len());
    let pool = state.0.lock().unwrap();
    if let Some(sess) = pool.get(&session_id) {
        match sess.tx.send(SshControlMsg::Data(data)) {
            Ok(_) => {
                println!("成功将数据送入 channel");
                Ok(())
            }
            Err(e) => {
                println!("发送数据到 channel 失败: {}", e);
                Err(e.to_string())
            }
        }
    } else {
        println!("找不到 SessionID: {}", session_id);
        Err("Session not found".to_string())
    }
}

#[tauri::command]
pub async fn resize_ssh_session(
    state: State<'_, SessionPool>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    let pool = state.0.lock().unwrap();
    if let Some(sess) = pool.get(&session_id) {
        sess.tx.send(SshControlMsg::Resize { cols, rows }).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Session not found".to_string())
    }
}
