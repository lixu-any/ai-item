use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Host {
    pub id: Option<i32>,
    pub name: String,
    pub group_id: Option<i32>,
    pub host: String,
    pub port: i32,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub tags: Option<String>,    // 逗号分隔的标签列表
    pub notes: Option<String>,   // 备注
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Group {
    pub id: Option<i32>,
    pub name: String,
    pub parent_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snippet {
    pub id: Option<i32>,
    pub name: String,
    pub command: String,
    pub group: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Credential {
    pub id: Option<i32>,
    pub name: String,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub description: Option<String>,
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
    
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;
    }

    let db_path = app_dir.join(DB_NAME);
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            parent_id INTEGER,
            FOREIGN KEY(parent_id) REFERENCES groups(id)
        )",
        [],
    ).map_err(|e| e.to_string())?;

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

    let _ = conn.execute("ALTER TABLE hosts ADD COLUMN private_key TEXT", []);
    let _ = conn.execute("ALTER TABLE hosts ADD COLUMN auth_token_id TEXT", []);

    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS snippets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            command TEXT NOT NULL,
            category TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS credentials (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            username TEXT NOT NULL,
            password TEXT,
            private_key TEXT,
            description TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS recent_connections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            host_id INTEGER NOT NULL,
            connected_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(host_id) REFERENCES hosts(id) ON DELETE CASCADE
        )",
        [],
    ).map_err(|e| e.to_string())?;

    // 迁移：为旧数据库动态添加新列
    let _ = conn.execute("ALTER TABLE hosts ADD COLUMN tags TEXT", []);
    let _ = conn.execute("ALTER TABLE hosts ADD COLUMN notes TEXT", []);

    Ok(())
}

#[tauri::command]
pub async fn add_host(app_handle: AppHandle, host: Host) -> Result<i32, String> {
    let conn = get_conn(&app_handle)?;
    conn.execute(
        "INSERT INTO hosts (name, group_id, host, port, username, password, private_key, tags, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        (
            &host.name,
            &host.group_id,
            &host.host,
            &host.port,
            &host.username,
            &host.password,
            &host.private_key,
            &host.tags,
            &host.notes,
        ),
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

#[tauri::command]
pub async fn get_hosts(app_handle: AppHandle) -> Result<Vec<Host>, String> {
    let conn = get_conn(&app_handle)?;
    let mut stmt = conn
        .prepare("SELECT id, name, group_id, host, port, username, password, private_key, tags, notes FROM hosts")
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
                tags: row.get(8)?,
                notes: row.get(9)?,
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
pub async fn update_host(app_handle: AppHandle, host: Host) -> Result<(), String> {
    let conn = get_conn(&app_handle)?;
    if let Some(id) = host.id {
        conn.execute(
            "UPDATE hosts SET name=?1, group_id=?2, host=?3, port=?4, username=?5, password=?6, private_key=?7, tags=?8, notes=?9 WHERE id=?10",
            (
                &host.name,
                &host.group_id,
                &host.host,
                &host.port,
                &host.username,
                &host.password,
                &host.private_key,
                &host.tags,
                &host.notes,
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
    conn.execute("UPDATE hosts SET group_id = NULL WHERE group_id = ?1", [id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM groups WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
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
pub async fn save_setting(app_handle: AppHandle, key: String, value: String) -> Result<(), String> {
    let conn = get_conn(&app_handle)?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        (&key, &value),
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_setting(app_handle: AppHandle, key: String) -> Result<Option<String>, String> {
    get_setting_internal(&app_handle, &key)
}

pub fn get_setting_internal(app_handle: &AppHandle, key: &str) -> Result<Option<String>, String> {
    let conn = get_conn(app_handle)?;
    let mut stmt = conn
        .prepare("SELECT value FROM settings WHERE key = ?1")
        .map_err(|e| e.to_string())?;
    let value = stmt.query_row([key], |row| row.get(0)).ok();
    Ok(value)
}

// Snippet commands
#[tauri::command]
pub async fn add_snippet(app_handle: AppHandle, name: String, command: String, category: Option<String>) -> Result<i32, String> {
    let conn = get_conn(&app_handle)?;
    conn.execute(
        "INSERT INTO snippets (name, command, category) VALUES (?1, ?2, ?3)",
        (&name, &command, &category),
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

#[tauri::command]
pub async fn get_snippets(app_handle: AppHandle) -> Result<Vec<Snippet>, String> {
    let conn = get_conn(&app_handle)?;
    let mut stmt = conn
        .prepare("SELECT id, name, command, category FROM snippets")
        .map_err(|e| e.to_string())?;
    let snippet_iter = stmt
        .query_map([], |row| {
            Ok(Snippet {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                command: row.get(2)?,
                group: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut snippets = Vec::new();
    for s in snippet_iter {
        snippets.push(s.map_err(|e| e.to_string())?);
    }
    Ok(snippets)
}

#[tauri::command]
pub async fn delete_snippet(app_handle: AppHandle, id: i32) -> Result<(), String> {
    let conn = get_conn(&app_handle)?;
    conn.execute("DELETE FROM snippets WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_snippet(app_handle: AppHandle, snippet: Snippet) -> Result<(), String> {
    if snippet.id.is_none() {
        return Err("Snippet ID is required for update".into());
    }
    let conn = get_conn(&app_handle)?;
    conn.execute(
        "UPDATE snippets SET name = ?1, command = ?2, category = ?3 WHERE id = ?4",
        (&snippet.name, &snippet.command, &snippet.group, snippet.id.unwrap()),
    ).map_err(|e| e.to_string())?;
    Ok(())
}

// Credential commands
#[tauri::command]
pub async fn add_credential(app_handle: AppHandle, cred: Credential) -> Result<i32, String> {
    let conn = get_conn(&app_handle)?;
    conn.execute(
        "INSERT INTO credentials (name, username, password, private_key, description) VALUES (?1, ?2, ?3, ?4, ?5)",
        (&cred.name, &cred.username, &cred.password, &cred.private_key, &cred.description),
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid() as i32)
}

#[tauri::command]
pub async fn get_credentials(app_handle: AppHandle) -> Result<Vec<Credential>, String> {
    let conn = get_conn(&app_handle)?;
    let mut stmt = conn
        .prepare("SELECT id, name, username, password, private_key, description FROM credentials")
        .map_err(|e| e.to_string())?;
    let cred_iter = stmt
        .query_map([], |row| {
            Ok(Credential {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                username: row.get(2)?,
                password: row.get(3)?,
                private_key: row.get(4)?,
                description: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut credentials = Vec::new();
    for c in cred_iter {
        credentials.push(c.map_err(|e| e.to_string())?);
    }
    Ok(credentials)
}

#[tauri::command]
pub async fn delete_credential(app_handle: AppHandle, id: i32) -> Result<(), String> {
    let conn = get_conn(&app_handle)?;
    conn.execute("DELETE FROM credentials WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_credential(app_handle: AppHandle, cred: Credential) -> Result<(), String> {
    let conn = get_conn(&app_handle)?;
    if let Some(id) = cred.id {
        conn.execute(
            "UPDATE credentials SET name=?1, username=?2, password=?3, private_key=?4, description=?5 WHERE id=?6",
            (&cred.name, &cred.username, &cred.password, &cred.private_key, &cred.description, id),
        ).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Credential ID missing".to_string())
    }
}

// ==================== 导入 / 导出 ====================

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportBundle {
    pub version: String,
    pub app: String,
    pub exported_at: String,
    pub groups: Vec<Group>,
    pub hosts: Vec<Host>,
    pub snippets: Vec<Snippet>,
    pub credentials: Option<Vec<Credential>>,
    pub settings: Option<Vec<(String, String)>>,
}

/// 导出数据：返回 JSON 字符串交由前端写文件
#[tauri::command]
pub async fn export_data(
    app_handle: AppHandle,
    include_credentials: bool,
    include_settings: bool,
) -> Result<String, String> {
    let conn = get_conn(&app_handle)?;

    // 分组
    let mut stmt = conn.prepare("SELECT id, name, parent_id FROM groups")
        .map_err(|e| e.to_string())?;
    let groups: Vec<Group> = stmt.query_map([], |row| {
        Ok(Group { id: Some(row.get(0)?), name: row.get(1)?, parent_id: row.get(2)? })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    // 主机
    let mut stmt = conn.prepare("SELECT id, name, group_id, host, port, username, password, private_key, tags, notes FROM hosts")
        .map_err(|e| e.to_string())?;
    let hosts: Vec<Host> = stmt.query_map([], |row| {
        Ok(Host {
            id: Some(row.get(0)?), name: row.get(1)?, group_id: row.get(2)?,
            host: row.get(3)?, port: row.get(4)?, username: row.get(5)?,
            password: row.get(6)?, private_key: row.get(7)?,
            tags: row.get(8)?, notes: row.get(9)?,
        })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    // 命令片段
    let mut stmt = conn.prepare("SELECT id, name, command, category FROM snippets")
        .map_err(|e| e.to_string())?;
    let snippets: Vec<Snippet> = stmt.query_map([], |row| {
        Ok(Snippet { id: Some(row.get(0)?), name: row.get(1)?, command: row.get(2)?, group: row.get(3)? })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();

    // 凭据（可选）
    let credentials = if include_credentials {
        let mut stmt = conn.prepare("SELECT id, name, username, password, private_key, description FROM credentials")
            .map_err(|e| e.to_string())?;
        let creds_vec: Vec<Credential> = stmt.query_map([], |row| {
            Ok(Credential {
                id: Some(row.get(0)?), name: row.get(1)?, username: row.get(2)?,
                password: row.get(3)?, private_key: row.get(4)?, description: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
        Some(creds_vec)
    } else {
        None
    };

    // 设置（可选，排除 AI Key）
    let settings = if include_settings {
        let mut stmt = conn.prepare("SELECT key, value FROM settings WHERE key != 'ai_api_key'")
            .map_err(|e| e.to_string())?;
        let settings_vec: Vec<(String, String)> = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
        Some(settings_vec)
    } else {
        None
    };

    let bundle = ExportBundle {
        version: "1".to_string(),
        app: "Nixu".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        groups,
        hosts,
        snippets,
        credentials,
        settings,
    };

    serde_json::to_string_pretty(&bundle).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub groups_added: usize,
    pub hosts_added: usize,
    pub snippets_added: usize,
    pub credentials_added: usize,
    pub settings_applied: usize,
    pub skipped: usize,
}

/// 导入数据：前端传入 JSON 字符串，跳过同名重复项
#[tauri::command]
pub async fn import_data(app_handle: AppHandle, json: String) -> Result<ImportResult, String> {
    let bundle: ExportBundle = serde_json::from_str(&json).map_err(|e| format!("解析失败: {}", e))?;
    let conn = get_conn(&app_handle)?;
    let mut result = ImportResult {
        groups_added: 0, hosts_added: 0, snippets_added: 0,
        credentials_added: 0, settings_applied: 0, skipped: 0,
    };

    // 构建 group 名称 → 新 id 映射（用于修复 host 的 group_id）
    let mut group_id_map: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();

    for g in &bundle.groups {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM groups WHERE name = ?1", [&g.name], |r| r.get(0)
        ).unwrap_or(0);
        if exists == 0 {
            conn.execute("INSERT INTO groups (name, parent_id) VALUES (?1, ?2)", (&g.name, &g.parent_id))
                .map_err(|e| e.to_string())?;
            let new_id = conn.last_insert_rowid() as i32;
            if let Some(old_id) = g.id {
                group_id_map.insert(old_id, new_id);
            }
            result.groups_added += 1;
        } else {
            // 已存在的 group，找到其 id 放入 map 以供 host 使用
            if let Some(old_id) = g.id {
                let existing_id: i32 = conn.query_row(
                    "SELECT id FROM groups WHERE name = ?1", [&g.name], |r| r.get(0)
                ).unwrap_or(old_id);
                group_id_map.insert(old_id, existing_id);
            }
            result.skipped += 1;
        }
    }

    for h in &bundle.hosts {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM hosts WHERE name = ?1 AND host = ?2", (&h.name, &h.host), |r| r.get(0)
        ).unwrap_or(0);
        if exists == 0 {
            let new_group_id = h.group_id.map(|old| *group_id_map.get(&old).unwrap_or(&old));
            conn.execute(
                "INSERT INTO hosts (name, group_id, host, port, username, password, private_key) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                (&h.name, &new_group_id, &h.host, &h.port, &h.username, &h.password, &h.private_key),
            ).map_err(|e| e.to_string())?;
            result.hosts_added += 1;
        } else {
            result.skipped += 1;
        }
    }

    for s in &bundle.snippets {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM snippets WHERE name = ?1", [&s.name], |r| r.get(0)
        ).unwrap_or(0);
        if exists == 0 {
            conn.execute(
                "INSERT INTO snippets (name, command, category) VALUES (?1, ?2, ?3)",
                (&s.name, &s.command, &s.group),
            ).map_err(|e| e.to_string())?;
            result.snippets_added += 1;
        } else {
            result.skipped += 1;
        }
    }

    if let Some(creds) = &bundle.credentials {
        for c in creds {
            let exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM credentials WHERE name = ?1", [&c.name], |r| r.get(0)
            ).unwrap_or(0);
            if exists == 0 {
                conn.execute(
                    "INSERT INTO credentials (name, username, password, private_key, description) VALUES (?1,?2,?3,?4,?5)",
                    (&c.name, &c.username, &c.password, &c.private_key, &c.description),
                ).map_err(|e| e.to_string())?;
                result.credentials_added += 1;
            } else {
                result.skipped += 1;
            }
        }
    }

    if let Some(settings) = &bundle.settings {
        for (k, v) in settings {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                (k, v),
            ).map_err(|e| e.to_string())?;
            result.settings_applied += 1;
        }
    }

    Ok(result)
}

/// 记录最近连接（连接成功时调用）
#[tauri::command]
pub async fn record_recent(app_handle: AppHandle, host_id: i32) -> Result<(), String> {
    let conn = get_conn(&app_handle)?;
    // 插入新记录
    conn.execute(
        "INSERT INTO recent_connections (host_id) VALUES (?1)",
        [host_id],
    ).map_err(|e| e.to_string())?;
    // 只保留最近 20 条
    conn.execute(
        "DELETE FROM recent_connections WHERE id NOT IN (
            SELECT id FROM recent_connections ORDER BY connected_at DESC LIMIT 20
        )",
        [],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// 返回最近连接的主机列表（去重，最多 5 条）
#[tauri::command]
pub async fn get_recents(app_handle: AppHandle) -> Result<Vec<Host>, String> {
    let conn = get_conn(&app_handle)?;
    let mut stmt = conn.prepare(
        "SELECT DISTINCT h.id, h.name, h.group_id, h.host, h.port, h.username,
                h.password, h.private_key, h.tags, h.notes
         FROM recent_connections rc
         JOIN hosts h ON h.id = rc.host_id
         ORDER BY rc.connected_at DESC
         LIMIT 5"
    ).map_err(|e| e.to_string())?;
    let iter = stmt.query_map([], |row| {
        Ok(Host {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            group_id: row.get(2)?,
            host: row.get(3)?,
            port: row.get(4)?,
            username: row.get(5)?,
            password: row.get(6)?,
            private_key: row.get(7)?,
            tags: row.get(8)?,
            notes: row.get(9)?,
        })
    }).map_err(|e| e.to_string())?;
    let hosts: Result<Vec<_>, _> = iter.map(|h| h.map_err(|e| e.to_string())).collect();
    hosts
}

/// 清空所有最近连接
#[tauri::command]
pub async fn clear_recents(app_handle: AppHandle) -> Result<(), String> {
    let conn = get_conn(&app_handle)?;
    conn.execute("DELETE FROM recent_connections", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}
