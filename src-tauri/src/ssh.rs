use ssh2::{Session, ErrorCode};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use base64::{Engine as _, engine::general_purpose};

pub enum SshControlMsg {
    Data(Vec<u8>),
    Resize { cols: u32, rows: u32 },
    Close,
}

pub struct SshSession {
    pub tx: mpsc::SyncSender<SshControlMsg>,
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
    cols: u32,
    rows: u32,
    connect_timeout_secs: Option<u64>,
    keepalive_secs: Option<u32>,
) -> Result<(), String> {
    let timeout_secs = connect_timeout_secs.unwrap_or(15);
    let addr = format!("{}:{}", host, port);

    // 带超时的 TCP 连接
    let tcp = std::net::TcpStream::connect_timeout(
        &addr.parse::<std::net::SocketAddr>()
             .map_err(|e| format!("解析地址失败：{}", e))?,
        Duration::from_secs(timeout_secs),
    ).map_err(|e| format!("连接超时或失败（{}s）：{}", timeout_secs, e))?;

    tcp.set_nonblocking(true).map_err(|e| e.to_string())?;

    let mut sess = Session::new().map_err(|e| e.to_string())?;
    sess.set_tcp_stream(tcp);
    sess.set_blocking(false);

    // 配置 Keepalive
    if let Some(interval) = keepalive_secs {
        if interval > 0 {
            sess.set_keepalive(true, interval);
        }
    }
    
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
        match channel.request_pty("xterm-256color", None, Some((cols, rows, 0, 0))) {
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

    let (tx, rx) = mpsc::sync_channel::<SshControlMsg>(2048);
    
    let session_id_clone = session_id.clone();
    let app_handle_clone = app_handle.clone();
    
    thread::spawn(move || {
        println!("Worker: 开始为 Session {} 启动循环...", session_id_clone);
        let mut buffer = [0u8; 8192];
        let mut idle_count = 0;
        'worker: loop {
            let mut read_active = false;
            let mut write_active = false;

            // 1. Read from SSH
            match channel.read(&mut buffer) {
                Ok(0) => {
                    println!("Worker: SSH channel read 遇到 EOF (Ok(0))");
                    break 'worker;
                }
                Ok(n) => {
                    read_active = true;
                    idle_count = 0;
                    let data = &buffer[..n];
                    let b64_data = general_purpose::STANDARD.encode(data);
                    if let Err(e) = app_handle_clone.emit(
                        &format!("sse-data-{}", session_id_clone),
                        b64_data,
                    ) {
                        println!("Worker: emit 发送事件失败: {}", e);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    println!("Worker: SSH channel 发生致命读取错误: {:?}", e);
                    break 'worker;
                }
            }

            // 2. Process Control Messages
            // Get the first message. If we just read from SSH, grab exactly what's pending (try_recv).
            // If idle, wait using recv_timeout.
            let first_msg_result = if read_active {
                rx.try_recv().map_err(|e| match e {
                    std::sync::mpsc::TryRecvError::Empty => std::sync::mpsc::RecvTimeoutError::Timeout,
                    std::sync::mpsc::TryRecvError::Disconnected => std::sync::mpsc::RecvTimeoutError::Disconnected,
                })
            } else {
                let timeout = if idle_count < 100 { 2 } else { 15 };
                rx.recv_timeout(Duration::from_millis(timeout))
            };

            let mut process_queue = Vec::new();
            match first_msg_result {
                Ok(msg) => {
                    process_queue.push(msg);
                    // Drain any remaining messages that queued up
                    while let Ok(m) = rx.try_recv() {
                        process_queue.push(m);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Normal idle empty queue
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    println!("Worker: Channel is disconnected. Worker thread exiting.");
                    break 'worker;
                }
            }

            // 3. Handle messages
            for msg in process_queue {
                write_active = true;
                idle_count = 0;
                match msg {
                    SshControlMsg::Data(data) => {
                        let mut written = 0;
                        while written < data.len() {
                            match channel.write(&data[written..]) {
                                Ok(n) => written += n,
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    thread::sleep(Duration::from_millis(10)); // Restored to 10ms for stability
                                }
                                Err(e) => {
                                    println!("Worker: 向 SSH channel 写入失败: {:?}", e);
                                    break; // breaks out of inner 'while written' loop
                                }
                            }
                        }
                        let _ = channel.flush();
                    }
                    SshControlMsg::Resize { cols, rows } => {
                        println!("Worker: 收到 Resize 请求, cols: {}, rows: {}", cols, rows);
                        loop {
                            match channel.request_pty_size(cols, rows, None, None) {
                                Ok(_) => break,
                                Err(e) if e.code() == ErrorCode::Session(EAGAIN) => {
                                    thread::sleep(Duration::from_millis(20)); // Restored to 20ms
                                }
                                Err(e) => {
                                    println!("Worker: PTY Size 同步失败: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    SshControlMsg::Close => {
                        println!("Worker: 收到 Close 请求，正在结束会话...");
                        let _ = channel.close();
                        break 'worker; // Properly breaks the worker loop
                    }
                }
            }

            if !read_active && !write_active {
                idle_count += 1;
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
    data: String,
) -> Result<(), String> {
    let pool = state.0.lock().unwrap();
    if let Some(sess) = pool.get(&session_id) {
        let bytes = general_purpose::STANDARD.decode(data.as_bytes()).unwrap_or_default();
        match sess.tx.send(SshControlMsg::Data(bytes)) {
            Ok(_) => {
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

#[tauri::command]
pub async fn close_ssh_session(
    state: State<'_, SessionPool>,
    session_id: String,
) -> Result<(), String> {
    let mut pool = state.0.lock().unwrap();
    if let Some(sess) = pool.remove(&session_id) {
        let _ = sess.tx.send(SshControlMsg::Close);
    }
    Ok(())
}

/// 执行系统监控命令并返回 JSON 字符串
#[tauri::command]
pub async fn get_host_stats(
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    private_key: Option<String>,
) -> Result<String, String> {
    use std::io::Read;

    let addr = format!("{}:{}", host, port);
    let tcp = std::net::TcpStream::connect_timeout(
        &addr.parse::<std::net::SocketAddr>().map_err(|e| e.to_string())?,
        std::time::Duration::from_secs(10),
    ).map_err(|e| format!("连接失败: {}", e))?;

    let mut sess = Session::new().map_err(|e| e.to_string())?;
    sess.set_tcp_stream(tcp);
    sess.set_blocking(true);
    sess.handshake().map_err(|e| e.to_string())?;

    if let Some(pk_raw) = private_key {
        if !pk_raw.is_empty() {
            let pkey = if pk_raw.trim().starts_with("-----BEGIN") {
                pk_raw
            } else {
                let path_str = pk_raw.replace("~", &std::env::var("HOME").unwrap_or_default());
                std::fs::read_to_string(path_str).map_err(|e| format!("私钥文件无效: {}", e))?
            };
            sess.userauth_pubkey_memory(&username, None, &pkey, password.as_deref())
                .map_err(|e| format!("密钥认证失败: {}", e))?;
        }
    }
    if !sess.authenticated() {
        if let Some(pw) = &password {
            sess.userauth_password(&username, pw)
                .map_err(|e| format!("密码认证失败: {}", e))?;
        }
    }
    if !sess.authenticated() {
        return Err("认证失败".to_string());
    }

    // 输出结构化 KEY=VALUE，便于前端解析
    let cmd = r#"
# OS
printf "OS=%s\n" "$(uname -sr)"

# Hostname
printf "HOSTNAME=%s\n" "$(hostname -s 2>/dev/null || hostname)"

# Uptime & load
UPTIME_STR=$(uptime 2>/dev/null)
printf "UPTIME_RAW=%s\n" "$UPTIME_STR"
LOAD=$(echo "$UPTIME_STR" | awk -F'load average[s]*: ' '{print $2}' | awk '{print $1}' | tr -d ',')
printf "LOAD1=%s\n" "${LOAD:-0}"

# CPU usage (Linux)
CPU_IDLE=$(top -bn1 2>/dev/null | grep -i "cpu(s)\|%cpu" | head -1 | awk '{for(i=1;i<=NF;i++) if($i~/id/) print $(i-1)}' | tr -d '%')
if [ -z "$CPU_IDLE" ]; then CPU_IDLE=0; fi
CPU_USED=$(awk "BEGIN{u=100-$CPU_IDLE; printf \"%.1f\", (u<0?0:u)}")
printf "CPU_USED=%s\n" "$CPU_USED"

# Memory (Linux: free -m)
MEM=$(free -m 2>/dev/null | grep "^Mem:")
if [ -n "$MEM" ]; then
    MEM_TOTAL=$(echo "$MEM" | awk '{print $2}')
    MEM_USED=$(echo "$MEM" | awk '{print $3}')
    printf "MEM_TOTAL=%s\n" "$MEM_TOTAL"
    printf "MEM_USED=%s\n" "$MEM_USED"
fi

# Disk /
DISK=$(df -m / 2>/dev/null | tail -1)
DISK_TOTAL=$(echo "$DISK" | awk '{print $2}')
DISK_USED=$(echo "$DISK" | awk '{print $3}')
DISK_PCT=$(echo "$DISK" | awk '{gsub(/%/,"",$5); print $5}')
printf "DISK_TOTAL=%s\n" "$DISK_TOTAL"
printf "DISK_USED=%s\n" "$DISK_USED"
printf "DISK_PCT=%s\n" "${DISK_PCT:-0}"

# Network
NET1=$(awk 'NR>2{rx+=$2; tx+=$10} END{print rx" "tx}' /proc/net/dev 2>/dev/null)
if [ -n "$NET1" ]; then
    sleep 1
    NET2=$(awk 'NR>2{rx+=$2; tx+=$10} END{print rx" "tx}' /proc/net/dev 2>/dev/null)
    RX1=$(echo "$NET1" | awk '{print $1}'); TX1=$(echo "$NET1" | awk '{print $2}')
    RX2=$(echo "$NET2" | awk '{print $1}'); TX2=$(echo "$NET2" | awk '{print $2}')
    printf "NET_RX_BPS=%s\n" "$((RX2-RX1))"
    printf "NET_TX_BPS=%s\n" "$((TX2-TX1))"
fi

# Top 5 CPU processes
TOP5=$(ps axo pid,pcpu,comm 2>/dev/null | sort -k2 -n -r | head -n 5 | awk '{printf "%s|%s|%s;", $1, $2, $3}')
printf "TOP5_PROCS=%s\n" "$TOP5"
"#;

    let mut channel = sess.channel_session().map_err(|e| e.to_string())?;
    channel.exec(cmd).map_err(|e| e.to_string())?;
    let mut output = String::new();
    channel.read_to_string(&mut output).map_err(|e| e.to_string())?;
    channel.wait_close().ok();

    Ok(output)
}
