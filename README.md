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
| **多会话标签页** | 本地终端与 SSH 会话统一管理，无限标签页切换 |
| **SSH 主机管理** | 分组树状视图，支持拖拽移动，主机增删改查 |
| **凭据中心** | 独立窗口管理用户名/密码/私钥，一键填入连接表单 |
| **SFTP 文件浏览器** | 终端标签页内直接切换文件管理视图，支持拖拽上传/下载 |
| **命令片段库 (Snippets)** | 保存常用长命令，侧边栏一键注入终端 |
| **广播模式** | 同时向所选多个会话发送相同命令，批量运维利器 |
| **会话录制与回放** | 录制终端操作为 Asciicast 格式，支持内置播放器回放 |
| **主机监控快照** | 一键获取服务器 CPU / 内存 / 磁盘 / 网络状态快照 |
| **Spotlight 快速连接** | `Cmd + K` 唤出全局搜索，快速连接最近主机或执行命令 |
| **Zmodem 文件传输** | 终端内通过 `rz` / `sz` 命令直接触发文件上传/下载弹窗 |
| **终端内搜索** | 支持关键词高亮搜索终端历史输出内容 |
| **字体与主题自定义** | 读取系统字体列表，自定义字号、行高、光标样式、渲染引擎（WebGL/DOM）|

---

## 🛠️ 技术栈

| 层级 | 技术选型 |
|------|----------|
| **UI 框架** | Vue 3 (Composition API) + TypeScript + Vite |
| **桌面外壳** | Tauri v2 |
| **终端引擎** | xterm.js 5.x + FitAddon / SearchAddon / WebglAddon |
| **SSH 连接** | Rust `ssh2` crate |
| **本地终端** | Rust `portable-pty` crate |
| **本地数据库** | `rusqlite`（纯 Rust SQLite，无 C 依赖交叉编译） |
| **AI 接入** | 兼容 OpenAI API 格式（支持 OpenAI / DeepSeek / 硅基流动 / 任意本地 Ollama 等） |

---

## ⚙️ AI 配置

进入 **设置 → AI 配置**，填写以下信息即可接入任意 OpenAI 兼容的大模型服务：

```
API 地址：https://api.deepseek.com/v1    # 或 OpenAI / 本地 Ollama 地址
API Key ：sk-xxxxxxxxxxxxxxxxxxxxxxxx
模型名称：deepseek-chat                   # 或 gpt-4o / qwen3 等
```

> 推荐使用 DeepSeek 或硅基流动，性价比最高，国内访问无需代理。

---

## 🚀 快速开始

### 环境依赖

1. **Node.js** >= 20.x（推荐 22.x）
2. **Rust** 稳定版工具链：
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

### 本地开发

```bash
# 安装前端依赖
npm install

# 启动开发模式（热更新）
npm run tauri dev
```

### 打包构建

```bash
# 当前平台
npm run tauri build

# macOS Universal（Intel + Apple Silicon）
npm run build:mac

# macOS Apple Silicon only
npm run build:mac-arm

# macOS Intel only
npm run build:mac-intel

# Windows（需配置 cargo-xwin 或使用 CI）
npm run build:win

# Linux
npm run build:linux
```

构建产物位于 `src-tauri/target/[target]/release/bundle/`。

---

## 📦 版本管理

版本号由以下三个文件共同控制，请保持一致：

1. `package.json`
2. `src-tauri/tauri.conf.json`
3. `src-tauri/Cargo.toml`

可使用 `npm version patch/minor/major` 辅助同步。

---

## 🗺️ Roadmap

- [ ] AI 错误自愈（命令失败时自动弹出修复建议，一键重试）
- [ ] 下一步命令预测（基于操作上下文主动推荐下一条命令）
- [ ] NLP 配置编辑（自然语言修改服务器配置文件）
- [ ] 多服务器状态对比（AI 差异分析两台服务器的配置与进程）
- [ ] 会话回放 AI 解说（录制回放时 AI 实时旁白每一步操作）
- [ ] AI Runbook 生成（将操作历史导出为结构化运维文档）

---

## 📜 设计理念

> **告别枯燥的传统终端，把开发者的主战场也做得赏心悦目。**
>
> Nixu 不只是一个终端模拟器，而是一个不断进化的 **AI 运维协处理器**。
> 它的目标是让记忆、搜索、诊断、防御这些消耗人类心智的重复劳动，都交给 AI 来承担；
> 而你，只需要专注于真正需要创造力的事。

---

## License

MIT © Nixu Terminal Contributors
