use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize)]
pub struct Host {
    pub id: Option<i32>,
    pub name: String,
    pub group_id: Option<i32>,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Group {
    pub id: Option<i32>,
    pub name: String,
    pub parent_id: Option<i32>,
}

const DB_NAME: &str = "aiterm.db";

fn get_conn(app_handle: &AppHandle) -> Result<Connection, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let db_path = app_dir.join(DB_NAME);
    Connection::open(db_path).map_err(|e| e.to_string())
}

pub fn init_db(app_handle: &AppHandle) -> Result<(), String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    
    // 确保应用数据目录存在
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;
    }

    let db_path = app_dir.join("aiterm.db");
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // 创建分组表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            parent_id INTEGER,
            FOREIGN KEY(parent_id) REFERENCES groups(id)
        )",
        [],
    ).map_err(|e| e.to_string())?;

    // 创建服务器表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS hosts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            group_id INTEGER,
            host TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 22,
            username TEXT NOT NULL,
            password TEXT,
            private_key TEXT,
            auth_token_id TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(group_id) REFERENCES groups(id)
        )",
        [],
    ).map_err(|e| e.to_string())?;

    // 尝试升级数据库，为已存在的 hosts 表添加 private_key 和 auth_token_id 字段
    let _ = conn.execute("ALTER TABLE hosts ADD COLUMN private_key TEXT", []);
    let _ = conn.execute("ALTER TABLE hosts ADD COLUMN auth_token_id TEXT", []);

    // 创建历史记录表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS history_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            host_id INTEGER,
            command TEXT NOT NULL,
            is_ai_generated BOOLEAN DEFAULT 0,
            ai_prompt TEXT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(host_id) REFERENCES hosts(id)
        )",
        [],
    ).map_err(|e| e.to_string())?;

    println!("数据库初始化成功: {:?}", app_dir.join(DB_NAME));
    Ok(())
}

#[tauri::command]
pub async fn add_host(app_handle: AppHandle, host: Host) -> Result<i32, String> {
    let conn = get_conn(&app_handle)?;
    conn.execute(
        "INSERT INTO hosts (name, group_id, host, port, username, password, private_key) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            &host.name,
            &host.group_id,
            &host.host,
            &host.port,
            &host.username,
            &host.password,
            &host.private_key,
        ),
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

#[tauri::command]
pub async fn get_hosts(app_handle: AppHandle) -> Result<Vec<Host>, String> {
    let conn = get_conn(&app_handle)?;
    let mut stmt = conn
        .prepare("SELECT id, name, group_id, host, port, username, password, private_key FROM hosts")
        .map_err(|e| e.to_string())?;
    let host_iter = stmt
        .query_map([], |row| {
            Ok(Host {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                group_id: row.get(2)?,
                host: row.get(3)?,
                port: row.get(4)?,
                username: row.get(5)?,
                password: row.get(6)?,
                private_key: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut hosts = Vec::new();
    for host in host_iter {
        hosts.push(host.map_err(|e| e.to_string())?);
    }
    Ok(hosts)
}

#[tauri::command]
pub async fn get_host(app_handle: AppHandle, id: i32) -> Result<Host, String> {
    let conn = get_conn(&app_handle)?;
    let mut stmt = conn
        .prepare("SELECT id, name, group_id, host, port, username, password, private_key FROM hosts WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let host = stmt
        .query_row([id], |row| {
            Ok(Host {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                group_id: row.get(2)?,
                host: row.get(3)?,
                port: row.get(4)?,
                username: row.get(5)?,
                password: row.get(6)?,
                private_key: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(host)
}

#[tauri::command]
pub async fn update_host(app_handle: AppHandle, host: Host) -> Result<(), String> {
    let conn = get_conn(&app_handle)?;
    if let Some(id) = host.id {
        conn.execute(
            "UPDATE hosts SET name=?1, group_id=?2, host=?3, port=?4, username=?5, password=?6, private_key=?7 WHERE id=?8",
            (
                &host.name,
                &host.group_id,
                &host.host,
                &host.port,
                &host.username,
                &host.password,
                &host.private_key,
                id,
            ),
        ).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Host ID missing".to_string())
    }
}

#[tauri::command]
pub async fn delete_host(app_handle: AppHandle, id: i32) -> Result<(), String> {
    let conn = get_conn(&app_handle)?;
    conn.execute("DELETE FROM hosts WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn add_group(app_handle: AppHandle, name: String, parent_id: Option<i32>) -> Result<i32, String> {
    let conn = get_conn(&app_handle)?;
    conn.execute(
        "INSERT INTO groups (name, parent_id) VALUES (?1, ?2)",
        (&name, &parent_id),
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

#[tauri::command]
pub async fn get_groups(app_handle: AppHandle) -> Result<Vec<Group>, String> {
    let conn = get_conn(&app_handle)?;
    let mut stmt = conn
        .prepare("SELECT id, name, parent_id FROM groups")
        .map_err(|e| e.to_string())?;
    let group_iter = stmt
        .query_map([], |row| {
            Ok(Group {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                parent_id: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut groups = Vec::new();
    for group in group_iter {
        groups.push(group.map_err(|e| e.to_string())?);
    }
    Ok(groups)
}
#[tauri::command]
pub async fn get_group(app_handle: AppHandle, id: i32) -> Result<Group, String> {
    let conn = get_conn(&app_handle)?;
    let mut stmt = conn
        .prepare("SELECT id, name, parent_id FROM groups WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let group = stmt
        .query_row([id], |row| {
            Ok(Group {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                parent_id: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(group)
}

#[tauri::command]
pub async fn update_group(app_handle: AppHandle, group: Group) -> Result<(), String> {
    let conn = get_conn(&app_handle)?;
    if let Some(id) = group.id {
        conn.execute(
            "UPDATE groups SET name=?1, parent_id=?2 WHERE id=?3",
            (&group.name, &group.parent_id, id),
        ).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Group ID missing".to_string())
    }
}

#[tauri::command]
pub async fn delete_group(app_handle: AppHandle, id: i32) -> Result<(), String> {
    let conn = get_conn(&app_handle)?;
    // 先将属于该分组的主机设为未分组
    conn.execute("UPDATE hosts SET group_id = NULL WHERE group_id = ?1", [id])
        .map_err(|e| e.to_string())?;
    // 再删除分组
    conn.execute("DELETE FROM groups WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
