<h1 align="center">
  <img src="docs/logo.png" width="72" alt="Nixu" /><br/>
  Nixu Terminal
</h1>

<p align="center">
  <strong>下一代 AI 原生 SSH 终端 —— 让服务器管理进入智能时代。</strong><br/>
  基于 Tauri 2 + Vue 3 + xterm.js 构建，深度融合大语言模型，让终端不只是命令行，而是你的智能运维助理。
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-2.x-blue?logo=tauri" />
  <img src="https://img.shields.io/badge/Vue-3-42b883?logo=vue.js" />
  <img src="https://img.shields.io/badge/Rust-stable-orange?logo=rust" />
  <img src="https://img.shields.io/badge/xterm.js-5.x-black" />
  <img src="https://img.shields.io/badge/License-MIT-green" />
</p>

---

## ✨ 核心特性

### 🤖 AI 时光机（Session Recap）
每次重新连接服务器时，Nixu 自动读取你过去的操作轨迹，通过 AI 生成一句精准的简报，悬浮在终端左上角：

> `✨ 检测到您上个栈周期主体为重构 Nginx 代理配置与拉取 Docker 镜像。`

**再也不用面对一个空白光标发呆**，你的上下文始终与你同在。

---

### 🔍 全息语义搜索（Holographic Search）
按下 `Cmd + Shift + R`，唤出聚光灯式搜索框，用自然语言召唤历史指令：

- _「上周查大文件的命令是什么来着？」_
- _「我今天这台机器都看了哪些网络端口？」_

AI 通过语义理解直接从该主机的历史日志中找到最匹配的命令，并**自动注入到当前光标位置**，按回车即可重现历史。

---

### 🛡️ 高危命令拦截（AI Security Guard）
输入 `rm -rf`、`chmod 777 /` 等危险命令时，Nixu 自动触发安全结界，弹出数学验证码，**强制你确认才能放行**。杜绝手抖删库跑路。

---

### 📊 NanoHUD 实时状态面板
SSH 连接后，终端右上角常驻一个极简 HUD（可关闭），实时显示目标服务器的：
- **CPU 使用率**（带迷你折线图）
- **内存使用率**（带迷你折线图）

每 5 秒自动轮询，数据完全不影响终端主体性能。

---

### 🎙️ AI 诊断侧边栏
右侧面板可唤出 AI 对话框，直接将终端错误输出发送给 AI，获得：
- 错误根因分析
- 修复命令建议
- 一键复制 / 注入到终端

---

### ⌨️ 智能自动补全
基于历史记录与当前命令上下文的实时补全提示，以幽灵文字形式显示在光标后方，Tab 键接受补全。

---

## 🚀 其他效率功能

| 功能 | 说明 |
|------|------|
| **多会话标签页** | 本地终端与 SSH 会话统一管理，无限标签页切换，独立进程隔离 |
| **SSH 主机管理** | 分组树状视图，支持拖拽移动，主机增删改查 |
| **凭据中心** | 独立窗口安全管理跨多主机的用户名、密码和私钥（文件/内联格式），一键填入 |
| **SFTP 极速文件浏览器** | 零心智负担内置 SFTP，与终端无缝切换，原生支持拖拽即刻上传 |
| **命令片段库 (Snippets)** | 保存常用长命令，侧栏结构化管理并一键安全注入终端 |
| **广播模式** | 对所选的多个服务器实例进行指令多路广播下发，集群运维神器 |
| **会话录制与回放** | 监听终端动作录制为标准的 Asciicast 格式，内建沉浸式黑客风播放器 |
| **Zmodem 支持** | 监听数据流特征，支持在终端直接输入 `rz`/`sz` 弹窗实现文件交互 |
| **全局快速唤醒** | `Cmd/Ctrl + K` 唤出 Spotlight 级全局搜索，快速定位最近使用的服务器 |

---

## 🏛️ 底层架构与语言规范 (Architecture)

Nixu Terminal 采用了严格的前后端分离与内存安全的架构设计，追求底层并发与终端级原生手感：

### 🧬 开发语言栈
- **Rust (后端系统层)**：使用强类型、无 GC 的系统级语言编写，直接接管 TCP 连接、并行 SSH 协商与伪终端 PTY 的系统调用，告别 Node.js 的内存泄漏问题。
- **TypeScript & Vue 3 (前端逻辑)**：纯 Composition API 构建，搭配 Web Worker / 异步回调结构，实现单线程环境下的零卡顿交互。
- **SQLite (持久层)**：内嵌数据库利用了没有任何 C++ 依赖的纯 Rust `rusqlite`，真正实现一次拷贝、跨平台完美运行。

### 🏎️ 极限低延时引擎设计 (Extreme Performance Pipeline)
我们重塑了整个终端的数据链路逻辑，以确保击键与屏幕视觉响应时间媲美系统级原生工具：
1. **Zero-Delay 底层发包**：通过 `TcpStream::set_nodelay(true)` 彻底废除默认的 Nagle 算法，消杀由于单字符打字封装造成的系统底层 40ms 网络延迟。
2. **黄金大页缓冲**：建立 `64KB` (65536 Byte) 的最大吞吐内存块与 `mpsc::sync_channel` 背压阻尼系统，在遭遇百兆宽带大文本拉取 (`cat huge.log`) 时保障 0 OOM (内存溢出) 崩溃。
3. **消除序列化灾难**：完全摒弃 Tauri IPC 默认耗时的字节数组 JSON `[ u8 ]` 转换；独创了基于全流通**无损 Base64** 结合 XtermJS 直接挂载方案，消除了 99% 的渲染瓶颈。
4. **渲染双引擎挂载**：检测显卡支持自动利用 WebGL 驱动极速绘制，显卡退化或不兼容时系统无缝重定向（Fallback）到基于底层 2D 加速的 `<canvas>`，彻底告别庞大 DOM 树造成的滚动卡顿。

### 💻 完美跨平台兼容 (Cross-Platform)
- **快捷键基因嗅探**：自带底层的 OS 检测，您在 Mac 上所有的原生肌肉记忆 `Cmd+K`、`Cmd+C`，放到 Windows 运行时会自动且完美地退化映射为 `Ctrl+K` 等响应。
- **本地终端智能适配**：`[cfg(target_os)]` 宏保障 Mac 下使用环境默认的 `/bin/zsh` 等壳层，而在 Windows 利用最新的 `ConPTY` 以原生的交互方式精准拉起 `cmd.exe`/`PowerShell`。
- **路径重排容错**：文件拖拽、SZ/RZ 的路径上传等完全兼容对 Mac 的 `/` 和 Windows 的 `\` 进行混合分割与校验。 

---

## ⚙️ AI 配置

进入 **设置 → AI 配置**，填写以下信息即可接入任意 OpenAI 兼容的大模型服务：

```
API 地址：https://api.deepseek.com/v1    # 或 OpenAI / 本地 Ollama 代理
API Key ：sk-xxxxxxxxxxxxxxxxxxxxxxxx
模型名称：deepseek-chat                   # 或 gpt-4o / qwen3 等
```

> 推荐使用 DeepSeek 或硅基流动，性价比最高，国内访问无需代理。

---

## 🚀 快速开始

### 预构建一键安装 (仅 Mac)
如果您只在意直接享受成品，请执行根目录附带的重型一键迁移构建清理脚本：
```bash
./install_mac.sh
```

### 开发模式与构建依赖
1. **Node.js** >= 20.x（推荐 22.x）
2. **Rust** 稳定版工具链

```bash
# 安装前端依赖 (含最新版 Xterm 引擎适配)
npm install

# 启动开发调试
npm run tauri dev

# 构建当前平台发行包
npm run tauri build
```

---

## 📦 版本管理

版本号同步分发在以下三个文件中（升级或打 Tag 时请协同确认）：
1. `package.json`
2. `src-tauri/tauri.conf.json`
3. `src-tauri/Cargo.toml`

您可以使用标准 `npm version patch/minor` 或自建脚本快速跨文件统一。

---

## 🗺️ Roadmap 未来路线图

- [ ] **AI 错误自愈**：命令失败检测 0 延迟，弹出修复建议并支持一键重试复原。
- [ ] **预测输入法**：基于工作链路上下文主动推荐下一条“极有可能输入”的高阶指令。
- [ ] **NLP 自然突变**：支持用类似 "把 Nginx 端口换成 8080" 的大白话直接修改远端服务器文本配置。
- [ ] **AI 运行手稿 (Runbook)**：结束一天的心智战斗后，将几小时的历史敲击归纳总结为可读的结构化 Markdown 运维交接文档。

---

## 📜 设计理念

> **告别枯燥的传统黑框终端，把开发者的主战场做到赏心悦目。**
>
> Nixu 不只是一个具备普通功能终端模拟器，而是一个不断进化的 **极客级 AI 协处理器**。
> 它的远景是：让记忆命令、无脑搜索、心智排错这些不产生化学反应的肉身劳动，全盘托付给 AI；让你，只需专注于挥洒真正需要创造力的艺术。

---

## License

MIT © Nixu Terminal Contributors
