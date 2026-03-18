use tauri::command;
use std::fs;

// 常见的基础命令 fallback list
const COMMON_CMDS: &[&str] = &[
    "ls -la", "cd ..", "pwd", "clear", "exit", "history",
    "git status", "git add -A", "git commit -m \"\"", "git pull", "git push",
    "docker ps", "docker-compose up -d", "docker images",
    "npm install", "npm run dev", "npm run build", "yarn dev", 
    "htop", "tail -f", "systemctl status", "systemctl restart"
];

#[command]
pub async fn get_completions(
    _session_id: String,
    current_input: String,
) -> Result<Vec<String>, String> {
    let input = current_input.trim().to_lowercase();
    if input.is_empty() {
        return Ok(vec![]);
    }

    let mut suggestions = Vec::new();

    // 1. 尝试读取本地的 bash/zsh history (先仅作按前缀匹配的基础解析)
    if let Some(home_dir) = dirs::home_dir() {
        let history_paths = vec![
            home_dir.join(".zsh_history"),
            home_dir.join(".bash_history"),
        ];

        for path in history_paths {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    for line in content.lines().rev() {
                        // zsh history format: ": 1609459200:0;ls -la"
                        let cmd = if line.starts_with(": ") {
                            let parts: Vec<&str> = line.splitn(2, ';').collect();
                            if parts.len() == 2 { parts[1] } else { line }
                        } else {
                            line
                        }.trim().to_string();

                        if !cmd.is_empty() && cmd.to_lowercase().starts_with(&input) && cmd.to_lowercase() != input {
                            if !suggestions.contains(&cmd) {
                                suggestions.push(cmd);
                                if suggestions.len() >= 5 {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if suggestions.len() >= 5 {
                break;
            }
        }
    }

    // 2. 如果 history 不够，补充内置的一些常见命令
    for &cmd in COMMON_CMDS {
        if suggestions.len() >= 5 {
            break;
        }
        let c = cmd.to_string();
        if c.to_lowercase().starts_with(&input) && c.to_lowercase() != input && !suggestions.contains(&c) {
            suggestions.push(c);
        }
    }

    Ok(suggestions)
}
