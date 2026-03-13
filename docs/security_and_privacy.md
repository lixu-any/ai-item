# 安全与隐私设计规范 (Security & Privacy)

## 1. SQLite 驱动策略 (针对用户约束)
用户明确要求：在 Go 环境下禁用交叉编译并使用纯 Go 驱动。
**在 Rust/Tauri 环境下的实施对策：**
*   **为何不使用交叉编译的动态库**: 为了避免在不同版本的 Linux/Windows 上出现 `libsqlite3.so` 缺失或版本不匹配的问题。
*   **Rust 解决方案**: 使用 `rusqlite` 配合 `bundled` 特性。
    *   `rusqlite = { version = "...", features = ["bundled"] }`
*   **效果**: 这会将 SQLite 的 C 源代码直接编译进 Rust 二进制文件中。它不依赖目标系统的 SQLite 动态库，在跨平台分发时具有极高的稳定性，达到了用户要求的“非交叉编译依赖”的效果。

---

## 2. AI 数据脱敏与隐私保护

### 2.1 敏感信息过滤 (Data Masking)
在将终端上下文发送给 AI 之前，客户端应进行本地扫描：
*   **IP 地址**: 模糊处理或替换为占位符（如 `[SERVER_IP]`）。
*   **环境变量**: 自动遮蔽包含 `KEY`, `SECRET`, `PASSWORD`, `TOKEN` 的变量。
*   **路径**: 可选脱敏用户宿主目录（如 `/Users/lixu/` 替换为 `~/`）。

### 2.2 离线模式 (Offline Toggle)
*   提供一个全局开关，一键断开所有 AI 相关请求，确保在处理极度敏感数据时，软件行为回归到纯净终端。

---

## 3. SSH 凭据本地保护
*   使用 **macOS Keychain** 和 **Windows Credential Manager** 存储敏感数据。
*   Rust 后端使用 `keyring` crate 与系统 API 深度集成。
*   数据库文件本身使用 `AES-256-GCM` 对敏感配置项（如 API Key）进行二次加密，密钥仅由系统钥匙串持有。

---

## 4. 全中文交互指南
*   **UI 语言**: 默认且优先使用中文。
*   **AI Prompt**: 显式告知 LLM 以中文回复（除非明确要求解释英文命令）。
*   **错误提示**: 将底层 SSH 错误码（如 `10061`）映射为友好的中文描述（如“目标机器积极拒绝连接”）。
