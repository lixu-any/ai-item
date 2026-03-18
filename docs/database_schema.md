# 数据库与存储设计文档 (Storage & Schema)

## 1. 存储选型
*   **数据库**: SQLite 3。
*   **Rust 驱动**: `rusqlite` (启用 `bundled` 特性)。
*   **ORM/Query**: 建议使用简单的 `rusqlite` 或 `sqlx` 以保持极致的跨平台编译稳定性。

---

## 2. 数据库表结构 (Schema)

### 2.1 `hosts` - 服务器连接信息
存储服务器的连接配置。
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `id` | INTEGER PRIMARY KEY | 自增 ID |
| `name` | TEXT | 别名 |
| `group_id` | INTEGER | 分组 ID (关联 groups 表) |
| `host` | TEXT | IP 或 域名 |
| `port` | INTEGER | 端口 (默认 22) |
| `username` | TEXT | 用户名 |
| `password` | TEXT | 密码 (开发前期明文存储) |
| `auth_token_id` | TEXT | 关联加密存储中的密钥标识 (预留) |
| `created_at` | DATETIME | 创建时间 |

### 2.2 `groups` - 分组管理
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `id` | INTEGER PRIMARY KEY | 自增 ID |
| `name` | TEXT | 分组名称 |
| `parent_id` | INTEGER | 父分组 ID (支持嵌套) |

### 2.3 `history_logs` - 命令历史与 AI 记录
| 字段 | 类型 | 说明 |
| :--- | :--- | :--- |
| `id` | INTEGER PRIMARY KEY | 自增 ID |
| `host_id` | INTEGER | 关联服务器 |
| `command` | TEXT | 执行的原始命令 |
| `is_ai_generated` | BOOLEAN | 是否为 AI 生成的命令 |
| `ai_prompt` | TEXT | 触发此命令的原始需求描述 |
| `timestamp` | DATETIME | 执行时间 |

---

## 3. 安全与加密方案

### 3.1 凭据存储 (Credentials)
**开发阶段说明：为了方便定位问题，前期密码将以明文形式存储在 `hosts` 表的 `password` 字段中。**
*   **后期规划 (方案 A - 推荐)**: 使用 Rust 库 `keyring-rs` 调用系统原生凭据管理器（macOS Keychain, Windows Credential Manager）。
*   **方案 B (备选)**: 程序初始化时生成一个随机的主密钥 (Master Key)，该密钥存储在系统钥匙串中。数据库中使用内容的主密钥进行 AES-256-GCM 加密存储。

### 3.2 数据备份
*   支持导出加密后的 JSON 备份文件。

---

## 4. 初始化配置 (App Config)
存储在 `config.toml` 或数据库配置表中：
*   AI 代理配置 (URL, API Key - 加密)。
*   终端字体、颜色主题。
*   自动锁屏等安全选项。
