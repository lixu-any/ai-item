use serde::{Deserialize, Serialize};
use ssh2::Session;
use tauri::Emitter;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::State;

pub struct SftpConnection {
    pub sess: Session,
}

#[derive(Default)]
pub struct SftpPool(pub Arc<Mutex<HashMap<String, SftpConnection>>>);

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modify_time: u64,
    pub permissions: u32,
}

#[derive(Debug, Serialize)]
pub struct SftpListResult {
    pub current_path: String,
    pub files: Vec<FileInfo>,
}

#[derive(Clone, Serialize)]
struct TransferProgress {
    session_id: String,
    file: String,
    transfer_type: String,
    transferred: u64,
    total: u64,
}

#[tauri::command]
pub async fn sftp_connect(
    state: State<'_, SftpPool>,
    session_id: String,
    host: String,
    port: u16,
    username: String,
    password: Option<String>,
    private_key: Option<String>,
) -> Result<(), String> {
    let mut pool = state.0.lock().unwrap();
    if pool.contains_key(&session_id) {
        return Ok(());
    }

    let addr = format!("{}:{}", host, port);
    let tcp = std::net::TcpStream::connect_timeout(
        &addr.parse::<std::net::SocketAddr>().map_err(|e| e.to_string())?,
        Duration::from_secs(15),
    ).map_err(|e| format!("SFTP连接超时: {}", e))?;

    let mut sess = Session::new().map_err(|e| e.to_string())?;
    sess.set_tcp_stream(tcp);
    sess.set_blocking(true);
    sess.handshake().map_err(|e| format!("握手失败: {}", e))?;

    // 认证
    if let Some(pk_raw) = private_key {
        let pkey = if pk_raw.trim().starts_with("-----BEGIN") {
            pk_raw
        } else {
            let path_str = pk_raw.replace("~", &std::env::var("HOME").unwrap_or_default());
            std::fs::read_to_string(path_str).map_err(|e| format!("私钥文件无效: {}", e))?
        };
        sess.userauth_pubkey_memory(&username, None, &pkey, password.as_deref())
            .map_err(|e| format!("私钥认证失败: {}", e))?;
    } else if let Some(pwd) = password {
        sess.userauth_password(&username, &pwd)
            .map_err(|e| format!("密码认证失败: {}", e))?;
    } else {
        return Err("需要密码或私钥".into());
    }

    pool.insert(session_id, SftpConnection { sess });
    Ok(())
}

#[tauri::command]
pub async fn sftp_read_dir(
    state: State<'_, SftpPool>,
    session_id: String,
    path: String,
) -> Result<SftpListResult, String> {
    let sess = {
        let pool = state.0.lock().unwrap();
        let conn = pool.get(&session_id).ok_or("SFTP会话未连接")?;
        conn.sess.clone()
    };

    let sftp = sess.sftp().map_err(|e| e.to_string())?;

    let dir_path = if path.is_empty() {
        // 如果路径为空，尝试获取用户的家目录或根目录
        let mut channel = sess.channel_session().map_err(|e| e.to_string())?;
        channel.exec("pwd").map_err(|e| e.to_string())?;
        use std::io::Read;
        let mut s = String::new();
        channel.read_to_string(&mut s).ok();
        let s = s.trim().to_string();
        if s.is_empty() { "/root".to_string() } else { s }
    } else {
        path
    };

    let dir_path_obj = std::path::Path::new(&dir_path);
    let entries = sftp.readdir(dir_path_obj).map_err(|e| e.to_string())?;

    let mut files = Vec::new();
    for (p, stat) in entries {
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        if name == "." || name == ".." {
            continue;
        }
        files.push(FileInfo {
            name,
            is_dir: stat.is_dir(),
            size: stat.size.unwrap_or(0),
            modify_time: stat.mtime.unwrap_or(0),
            permissions: stat.perm.unwrap_or(0),
        });
    }

    // 默认排序：文件夹在前，文件名排序
    files.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(SftpListResult {
        current_path: dir_path,
        files,
    })
}

#[tauri::command]
pub async fn sftp_mkdir(
    state: State<'_, SftpPool>,
    session_id: String,
    path: String,
) -> Result<(), String> {
    let sess = {
        let pool = state.0.lock().unwrap();
        pool.get(&session_id).ok_or("SFTP会话未连接")?.sess.clone()
    };
    let sftp = sess.sftp().map_err(|e| e.to_string())?;

    let path_obj = std::path::Path::new(&path);
    sftp.mkdir(path_obj, 0o755).map_err(|e| format!("创建目录失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn sftp_delete(
    state: State<'_, SftpPool>,
    session_id: String,
    path: String,
    is_dir: bool,
) -> Result<(), String> {
    let sess = {
        let pool = state.0.lock().unwrap();
        pool.get(&session_id).ok_or("SFTP会话未连接")?.sess.clone()
    };
    let sftp = sess.sftp().map_err(|e| e.to_string())?;

    let path_obj = std::path::Path::new(&path);
    if is_dir {
        sftp.rmdir(path_obj).map_err(|e| format!("删除目录失败: {}", e))?;
    } else {
        sftp.unlink(path_obj).map_err(|e| format!("删除文件失败: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn sftp_rename(
    state: State<'_, SftpPool>,
    session_id: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    let sess = {
        let pool = state.0.lock().unwrap();
        pool.get(&session_id).ok_or("SFTP会话未连接")?.sess.clone()
    };
    let sftp = sess.sftp().map_err(|e| e.to_string())?;

    let old = std::path::Path::new(&old_path);
    let new = std::path::Path::new(&new_path);
    sftp.rename(old, new, None).map_err(|e| format!("重命名/移动失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn sftp_download_file(
    app: tauri::AppHandle,
    state: State<'_, SftpPool>,
    session_id: String,
    remote_path: String,
    local_path: String,
) -> Result<(), String> {
    let sess = {
        let pool = state.0.lock().unwrap();
        pool.get(&session_id).ok_or("SFTP会话未连接")?.sess.clone()
    };

    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let remote_obj = std::path::Path::new(&remote_path);
        
        let local_obj = std::path::Path::new(&local_path);
        let mut local_file = std::fs::File::create(local_obj)
            .map_err(|e| format!("无法创建本地文件: {}", e))?;

        let (mut remote_file, stat) = sess.scp_recv(remote_obj)
            .map_err(|e| format!("打开远程文件失败(建议尝试): {}", e))?;
        let total = stat.size();
        let file_name = remote_obj.file_name().unwrap_or_default().to_string_lossy().into_owned();
        
        let mut buffer = vec![0u8; 128 * 1024]; // 128KB 吞吐量缓冲区在堆上分配
        let mut transferred = 0u64;
        let mut last_emit = std::time::Instant::now();

        use std::io::{Read, Write};
        loop {
            let n = remote_file.read(&mut buffer).map_err(|e| format!("读取失败: {}", e))?;
            if n == 0 { break; }
            local_file.write_all(&buffer[..n]).map_err(|e| format!("写入失败: {}", e))?;
            transferred += n as u64;

            if last_emit.elapsed().as_millis() > 100 {
                app.emit("sftp-progress", TransferProgress {
                    session_id: session_id.clone(),
                    file: file_name.clone(),
                    transfer_type: "download".into(),
                    transferred,
                    total,
                }).ok();
                last_emit = std::time::Instant::now();
            }
        }
        
        app.emit("sftp-progress", TransferProgress {
            session_id: session_id.clone(),
            file: file_name,
            transfer_type: "download".into(),
            transferred,
            total,
        }).ok();

        Ok(())
    }).await.map_err(|e| format!("线程执行失败: {}", e))??;

    Ok(())
}

#[tauri::command]
pub async fn sftp_upload_file(
    app: tauri::AppHandle,
    state: State<'_, SftpPool>,
    session_id: String,
    local_path: String,
    remote_path: String,
) -> Result<(), String> {
    let sess = {
        let pool = state.0.lock().unwrap();
        pool.get(&session_id).ok_or("SFTP会话未连接")?.sess.clone()
    };

    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let local_obj = std::path::Path::new(&local_path);
        let mut local_file = std::fs::File::open(local_obj)
            .map_err(|e| format!("无法打开本地文件: {}", e))?;

        let remote_obj = std::path::Path::new(&remote_path);
        let total = local_file.metadata().ok().map(|m| m.len()).unwrap_or(0);
        let mut remote_file = sess.scp_send(remote_obj, 0o644, total, None)
            .map_err(|e| format!("创建远程传输通道失败: {}", e))?;
        
        let file_name = local_obj.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let mut buffer = vec![0u8; 128 * 1024]; // 128KB 缓冲区
        let mut transferred = 0u64;
        let mut last_emit = std::time::Instant::now();

        use std::io::{Read, Write};
        loop {
            let n = local_file.read(&mut buffer).map_err(|e| format!("读取失败: {}", e))?;
            if n == 0 { break; }
            remote_file.write_all(&buffer[..n]).map_err(|e| format!("写入失败: {}", e))?;
            transferred += n as u64;

            if last_emit.elapsed().as_millis() > 100 {
                app.emit("sftp-progress", TransferProgress {
                    session_id: session_id.clone(),
                    file: file_name.clone(),
                    transfer_type: "upload".into(),
                    transferred,
                    total,
                }).ok();
                last_emit = std::time::Instant::now();
            }
        }
        
        remote_file.send_eof().ok();
        remote_file.wait_eof().ok();
        remote_file.close().ok();
        remote_file.wait_close().ok();

        app.emit("sftp-progress", TransferProgress {
            session_id: session_id.clone(),
            file: file_name,
            transfer_type: "upload".into(),
            transferred,
            total,
        }).ok();

        Ok(())
    }).await.map_err(|e| format!("线程执行失败: {}", e))??;

    Ok(())
}

#[tauri::command]
pub async fn sftp_close(
    state: State<'_, SftpPool>,
    session_id: String,
) -> Result<(), String> {
    let mut pool = state.0.lock().unwrap();
    pool.remove(&session_id);
    Ok(())
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace("'", "'\\''"))
}

#[tauri::command]
pub async fn sftp_compress(
    state: State<'_, SftpPool>,
    session_id: String,
    parent_path: String,
    target_name: String,
) -> Result<(), String> {
    let sess = {
        let pool = state.0.lock().unwrap();
        let conn = pool.get(&session_id).ok_or("SFTP会话未连接")?;
        conn.sess.clone()
    };
    
    let mut channel = sess.channel_session().map_err(|e| e.to_string())?;
    
    let cmd = format!(
        "cd {} && tar -czf {} {}", 
        shell_escape(&parent_path), 
        shell_escape(&format!("{}.tar.gz", target_name)), 
        shell_escape(&target_name)
    );
    
    channel.exec(&cmd).map_err(|e| e.to_string())?;
    
    use std::io::Read;
    let mut stderr = String::new();
    channel.stderr().read_to_string(&mut stderr).ok();
    
    channel.wait_close().map_err(|e| e.to_string())?;
    let exit_status = channel.exit_status().unwrap_or(1);
    
    if exit_status != 0 {
        return Err(format!("压缩失败: {}", stderr));
    }
    
    Ok(())
}

#[tauri::command]
pub async fn sftp_extract(
    state: State<'_, SftpPool>,
    session_id: String,
    parent_path: String,
    target_name: String,
) -> Result<(), String> {
    let sess = {
        let pool = state.0.lock().unwrap();
        let conn = pool.get(&session_id).ok_or("SFTP会话未连接")?;
        conn.sess.clone()
    };
    
    let is_zip = target_name.to_lowercase().ends_with(".zip");
    let is_tar_gz = target_name.to_lowercase().ends_with(".tar.gz") || target_name.to_lowercase().ends_with(".tgz");
    let is_tar = target_name.to_lowercase().ends_with(".tar");
    
    if !is_zip && !is_tar_gz && !is_tar {
        return Err("不支持的压缩包格式".to_string());
    }

    let mut channel = sess.channel_session().map_err(|e| e.to_string())?;
    
    let extract_cmd = if is_zip {
        format!("unzip -o {}", shell_escape(&target_name))
    } else if is_tar_gz {
        format!("tar -xzf {}", shell_escape(&target_name))
    } else {
        format!("tar -xf {}", shell_escape(&target_name))
    };

    let cmd = format!("cd {} && {}", shell_escape(&parent_path), extract_cmd);
    
    channel.exec(&cmd).map_err(|e| e.to_string())?;
    
    use std::io::Read;
    let mut stderr = String::new();
    channel.stderr().read_to_string(&mut stderr).ok();
    
    channel.wait_close().map_err(|e| e.to_string())?;
    let exit_status = channel.exit_status().unwrap_or(1);
    
    if exit_status != 0 {
        return Err(format!("解压失败: {}", stderr));
    }
    
    Ok(())
}
