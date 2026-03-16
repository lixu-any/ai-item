<h1 align="center">Nixu Terminal</h1>

<p align="center">
  <strong>一款基于 Web 技术构建的现代化、高颜值智能终端。</strong><br>
  提供流畅的本地终端体验与强大的 SSH 管理功能，同时深度集成智能辅助能力。
</p>

## ✨ 特性

- **现代化架构**：基于 Tauri 2 + Vue 3 + xterm.js 构建，同时拥有原生应用的性能与 Web 应用的灵活性。
- **高颜值界面**：采用毛玻璃特效和精致的深浅主题设计，每一次敲击键盘都是视觉享受。
- **强大的 SSH 管理**：
  - 支持主机分组管理、树状视图组织。
  - 内置凭据中心，支持密码与 SSH 私钥认证。
  - 标签页（Tab）无缝切换多个本地或远程会话。
- **智能辅助 (AI)**：
  - 内置 AI 侧边栏，支持对接 OpenAI / DeepSeek / 硅基流动 等兼容 API。
  - 智能终端命令建议与自动补全（基于本地历史或快捷命令）。
- **效率工具**：
  - **终端分屏与广播**：同时向多个会话广播命令。
  - **命令片段 (Snippets)**：保存常用长命令，一键点击执行。
  - **监控面板**：实时获取目标服务器的 CPU、内存、磁盘等运行状态快照。
  - **会话录制与回放**：支持将终端操作录制为 Asciicast 格式，并在内部播放。
  - **Spotlight 快速搜索**：全局快捷键唤出执行命令或快速连接。
- **灵活的个性化设置**：
  - 支持调整终端字体（读取本机系统字体）、字号、行高。
  - 自定义光标样式与标签页显示模式。

## 🛠️ 技术栈

* **前端**：[Vue 3](https://vuejs.org/) (Composition API), [Vite](https://vitejs.dev/), [TypeScript](https://www.typescriptlang.org/)
* **终端核心**：[xterm.js](https://xtermjs.org/) 及常用 Addon (Fit, Search, WebGL)
* **后端**：[Rust](https://www.rust-lang.org/), [Tauri v2](https://v2.tauri.app/)
* **核心依赖**：`portable-pty` (本地终端), `ssh2` (SSH连接), `rusqlite` (本地数据库)

## 🚀 快速开始

### 环境依赖
确保你已经安装了以下环境：
1. **Node.js** (>= 20.x, 推荐 22.x) 及包管理器 **npm** / **yarn**。
2. **Rust** 工具链。如果没有安装，可运行：`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

### 下载与运行

#### 1. 克隆项目 & 安装依赖
```bash
# 1. 切换到项目根目录
cd lidayeiTerm

# 2. 安装前端依赖
npm install
```

#### 2. 本地开发与调试
启动开发服务器，并自动打开带有热更新机制的 Tauri 应用窗口：
```bash
npm run tauri dev
```

#### 3. 构建打包
如果针对**当前正在使用的操作系统**打包应用：
```bash
npm run tauri build
```

为支持**跨平台打包和指令参数**，项目内置了以下便捷命令：
```bash
# 构建 macOS M1/M2/M3 (Apple Silicon) + Intel 的通用架构包 （最推荐 Mac 用户使用此命令）
npm run build:mac

# 仅构建 macOS Intel 架构处理器版本
npm run build:mac-intel

# 仅构建 macOS ARM (M系列芯片) 架构版本
npm run build:mac-arm

# 构建 Windows 版本 (需要配置或宿主机支持 target `x86_64-pc-windows-msvc`)
npm run build:win

# 构建 Linux 版本 (需要配置或宿主机支持 target `x86_64-unknown-linux-gnu`)
npm run build:linux

# ⚠️ 尝试一次性构建上述所有平台的包（需要本机已配置好完整的跨平台编译工具链支持）
npm run build:all
```
> **跨平台交叉编译提示 (Cross-Compilation)**:
> 默认情况下，Tauri 在 macOS 上只能打包 `.app` 和 `.dmg`，在 Windows 只能打包 `.exe` 和 `.msi`。
> 若要在一台 Mac 上运行 `npm run build:win` 成功打包 Windows 的程序，你需要提前配置 Rust 的交叉编译工具链（如 `cargo-xwin`）或者使用 [GitHub Actions 自动化 CI 流水线](https://tauri.app/v1/guides/building/github-actions/) 在云端进行多环境打包。

构建成功后，对应的二进制可执行文件及安装包会生成在 `src-tauri/target/[架构Target]/release/bundle/` 目录中。

## 📦 版本控制
本应用的版本号由以下三个文件共同控制，请确保它们保持一致（或者使用 `npm version [patch/minor/major]` 命令自动同步更新）：
1. `/package.json`
2. `/src-tauri/tauri.conf.json`
3. `/src-tauri/Cargo.toml`

---

> **设计理念：**
> 告别枯燥乏味的传统终端，将开发者的效率工具也做得赏心悦目。Nixu 在注重极简设计和良好交互体验的同时，正在不断探索将大型语言模型无缝融入日常的运维和编码工作流中。
