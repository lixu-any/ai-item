# Nixu 跨平台兼容性分析

> 覆盖平台：**macOS · Linux · Windows**

---

## 一览表

| 功能模块 | macOS | Linux | Windows | 说明 |
|----------|:-----:|:-----:|:-------:|------|
| SSH 连接 | ✅ | ✅ | ✅ | `ssh2` crate 跨平台 |
| 本地终端（PTY）| ✅ | ✅ | ✅⚠️ | 已修复 shell 检测（见下） |
| 数据库（SQLite）| ✅ | ✅ | ✅ | `rusqlite` bundled 纯 Rust |
| AI 助手 | ✅ | ✅ | ✅ | `reqwest` HTTP 跨平台 |
| 导入/导出 | ✅ | ✅ | ✅ | Tauri `plugin-fs` + `plugin-dialog` |
| 应用设置 | ✅ | ✅ | ✅ | KV 存 SQLite |
| 窗口管理 | ✅ | ✅ | ✅ | Tauri WebviewWindow |
| 字体预设 | ✅ | ⚠️ | ⚠️ | 部分字体 macOS 独占（见下） |
| 图标 | ✅ | ✅ | ✅ | `tauri icon` 已生成各平台格式 |

---

## 已修复问题

### 1. ✅ PTY 本地终端 Shell 检测（`pty.rs`）

**问题**：原代码读取 `$SHELL` 环境变量，fallback 为 `/bin/zsh`。Windows 上 `$SHELL` 不存在，且 `/bin/zsh` 路径无效，导致本地终端无法启动。

**修复**：使用 `#[cfg]` 条件编译，Windows 读取 `%COMSPEC%`（通常为 `cmd.exe`），Unix 系统读取 `$SHELL` fallback 为 `/bin/sh`。

```rust
#[cfg(target_os = "windows")]
let shell = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
#[cfg(not(target_os = "windows"))]
let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
```

---

## 潜在风险（无需立即修复）

### 2. ⚠️ 字体预设中的平台专属字体

字体设置中预设了以下字体，在不同平台上可能缺失：

| 字体 | macOS | Linux | Windows |
|------|:-----:|:-----:|:-------:|
| Cascadia Code | ❌需手动安装 | ❌需手动安装 | ✅ Win11 内置 |
| JetBrains Mono | ❌需手动安装 | ❌需手动安装 | ❌需手动安装 |
| Fira Code | ❌需手动安装 | ❌需手动安装 | ❌需手动安装 |
| **SF Mono** | ✅ 内置 | ❌ 不存在 | ❌ 不存在 |
| **Menlo** | ✅ 内置 | ❌ 不存在 | ❌ 不存在 |
| Courier New | ✅ | ✅ | ✅ 内置 |

**影响**：字体缺失时 xterm.js 会自动 fallback 到 `monospace`，功能不受影响，但显示效果与预期不同。

**建议**：在字体预设按钮上加 `(macOS)` 等平台标注，或将 SF Mono / Menlo 替换为更通用的字体。

### 3. ⚠️ Linux 下 `keyring` crate 可能需要额外依赖

项目依赖了 `keyring` crate（尽管目前凭据已改为存储在 SQLite），在 Linux 上需要 `libsecret` 或 KWallet。如果 `keyring` 仍在 Cargo.toml 中但未被使用，建议移除以减少依赖复杂度。

### 4. ⚠️ Windows 下终端 ANSI 颜色

`TERM=xterm-256color` 已正确设置，Windows 上 ConPTY（portable-pty 使用）支持 ANSI。但部分老版本 Windows 10 可能存在兼容问题，建议在 Windows 上测试验证。

---

## 无问题的模块

- **SSH**：`ssh2` / `libssh2` 完全跨平台，Windows 静态链接 OpenSSL
- **数据库路径**：Tauri 的 `app_data_dir()` 在三平台返回正确路径（`~/Library/Application Support`、`~/.local/share`、`%APPDATA%`）
- **文件导入导出**：`tauri-plugin-fs` + `tauri-plugin-dialog` 原生支持三平台
- **AI 请求**：`reqwest` 使用 TLS，三平台均可
- **Tauri WebView**：macOS 用 WKWebView，Linux 用 WebKitGTK，Windows 用 Edge WebView2

---

## 建议后续操作

1. **Windows 真机测试**：本地终端（PTY）和 SSH 功能需在 Windows 上实际验证
2. **字体标注**：在 SF Mono / Menlo 预设按钮上加 `(仅 macOS)` 提示
3. **移除未用的 `keyring` 依赖**（如不再使用）
