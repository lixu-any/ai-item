# 技术架构设计文档 (Technical Architecture)

## 1. 核心架构模式
本项目采用 **Tauri v2** 框架，分为前端渲染层（Frontend）和后端逻辑层（Backend/Core）。

### 1.1 职责分工
*   **Frontend (Vue/React)**: 负责 UI 渲染、终端显示 (xterm.js)、用户输入捕获、AI 对话界面。
*   **Backend (Rust)**: 负责进程间通信 (IPC)、SSH 协议实现、文件系统操作、本地数据库管理 (SQLite)、AI API 请求中转。

---

## 2. 后端模块设计 (Rust)

### 2.1 会话管理器 (Session Manager)
*   **职责**: 管理所有活动的 SSH 连接。
*   **设计**: 使用一个全局的 `Arc<Mutex<HashMap<SessionId, Session>>>` 来持有活动连接。
*   **并发**: 每一个 SSH 频道 (Channel) 在 Rust 侧运行在一个独立的 `tokio` 协程中，确保高频输出不会阻塞主线程。

### 2.2 IPC 通信协议
*   **Command 模式**: 前端通过 `invoke('cmd_name', data)` 调用后端。
*   **Event 模式**: 后端通过 `Window::emit` 主动向前端推送数据（如：终端字符流、实时监控指标、AI 生成进度）。

### 2.3 AI 引擎层 (AI Engine)
*   **抽象层**: 定义 `Trait AIProvider`，实现对不同服务商（DeepSeek, OpenAI, Ollama）的统一封装。
*   **流式处理**: 支持 Server-Sent Events (SSE)，实现 AI 响应的逐字流式显示。

---

## 3. 关键交互流程

### 3.1 终端数据流
1.  用户在 `xterm.js` 输入字符。
2.  前端通过 IPC 发送至 Rust。
3.  Rust 写入 `russh` 的标准输入。
4.  远程服务器返回数据，Rust 捕获输出。
5.  Rust 通过监听器将字符流回传给前端 `emit` 事件。

### 3.2 AI 命令执行流
1.  用户输入自然语言。
2.  Rust 收集当前环境上下文（当前目录、操作系统类型、近期命令记录）。
3.  Rust 封装系统 Prompt 并调用 AI API。
4.  返回命令后，前端显示为“待确认”状态。
5.  用户点击执行后，Rust 侧通过已有的 SSH 会话发送该命令。

---

## 4. 性能与优化
*   **零拷贝渲染**: 尽量减少大批量字符传输时的中间拷贝。
*   **资源限制**: 限制单个会话的缓冲区大小，防止内存暴涨。
*   **离线优先**: 基本会话管理和数据库读取必须在无网络环境下秒开。
