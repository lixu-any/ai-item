# Nixu 跨平台编译指南 —— GitHub Actions 自动化构建

由于 macOS 无法直接通过交叉编译打包出依赖复杂系统 SDK 的 Windows 程序（由于缺少 `MSVC Linker`、Windows 原生 SDK 和 `WebView2`），**最推荐、最稳妥、最免折腾的方案是使用 GitHub 免费提供的云端服务器来为您编译。**

通过 Github Actions，你可以配置一个工作流，同时无脑并行打包出 Windows (`.msi` / `.exe`)、macOS (`.dmg` / `.app`) 以及 Linux (`.deb` / `.AppImage`) 的安装包。

以下是具体的实施步骤：

## 步骤 1：在项目中创建 Workflow 文件

在你的项目根目录下，创建一个 `.github` 文件夹，然后在其中再创建一个 `workflows` 文件夹。
最后新建一个名为 `release.yml` 的文件，即完整路径为：
`.github/workflows/release.yml`

## 步骤 2：写入自动化构建配置 (YAML)

将以下配置代码完整复制粘贴到 `release.yml` 中。这份配置会在你每次往 GitHub 仓库打上 `v` 开头的标签（如 `v1.0.0`）并推送时，自动触发跨平台编译。

```yaml
name: "Nixu Release Build"

on:
  push:
    tags:
      - 'v*' # 只有当你推送形如 v1.0.2 的 tag 时才会触发

jobs:
  build-tauri:
    strategy:
      fail-fast: false
      matrix:
        include:
          # Windows 构建任务
          - platform: 'windows-latest'
            args: ''
          # macOS Intel 构建任务
          - platform: 'macos-latest'
            args: '--target x86_64-apple-darwin'
          # macOS M系列芯片 (Apple Silicon) 构建任务
          - platform: 'macos-latest'
            args: '--target aarch64-apple-darwin'
          # Linux 构建任务 (如需去重也可删掉)
          - platform: 'ubuntu-20.04'
            args: ''

    runs-on: ${{ matrix.platform }}
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      # 针对 Linux 平台安装特有系统依赖
      - name: install dependencies (ubuntu only)
        if: matrix.platform == 'ubuntu-20.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf

      # 安装 Node.js
      - name: setup node
        uses: actions/setup-node@v4
        with:
          node-version: 18

      # 安装 Rust (使用最新的稳定版)
      - name: install Rust stable
        uses: dtolnay/rust-toolchain@stable
        with:
          # 对于 macOS 交叉编译需要添加特定的 target
          targets: ${{ matrix.platform == 'macos-latest' && 'aarch64-apple-darwin,x86_64-apple-darwin' || '' }}

      # 安装项目 NPM 依赖
      - name: install app dependencies
        run: npm install

      # 核心：使用 Tauri Action 执行真正的 build 打包
      # 它会自动拉取 tauri-cli 并运行 npm run tauri build
      - name: build tauri app
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: app-v__VERSION__ # 将当前版本号作为 Tag
          releaseName: 'Nixu v__VERSION__'
          releaseBody: '详见提交说明'
          releaseDraft: true     # 生成的 release 会以草稿状态存在，你可以手动检查后再发布
          prerelease: false
          args: ${{ matrix.args }}
```

## 步骤 3：提交并推送到 GitHub

在本地终端中，通过 Git 将刚才增加的 Workflow 文件推送到你的 GitHub 仓库中：

```bash
git add .
git commit -m "chore: 增加跨平台构建 GitHub Actions 配置"
git push origin HEAD
```

*如果你还没有在 GitHub 上建立私有仓库，请先在网页端建立，并将本地代码绑定且推给它。*

## 步骤 4：触发自动打包

当你准备好要发布新版本给 Windows 用户时，只需要给刚才提交的代码打上一个 `v` 开头的标签：

```bash
# 假设当前要发布的版本是 1.0.1
git tag v1.0.1
git push origin v1.0.1
```

## 步骤 5：下载打包好的 Windows 安装包

1. GitHub 接收到携带 Tag 的 `push` 请求后，会自动为你启动不同操作系统的 4 台虚拟机。
2. 打开你项目的 GitHub 主页，点击横向菜单栏中的 **Actions** 选项卡。
3. 你会看到一个黄圈正在运行的 `Nixu Release Build` 工作流。它的执行过程大约会持续 **10 ~ 15 分钟**（因为首次运行要编译很多 C++ 网络库、安全库和 Rust 代码）。
4. 变为绿色对钩后，点击右侧边栏的 **Releases**（或者在项目首页右侧的 Releases 版块）。
5. 你会发现系统自动为你用草稿形态创建了一个 Release 页面。点进去就能看到打包生成的几个安装包了，包含：
    * `Nixu_x.x.x_x64_en-US.msi` / `.exe` (Windows 安装包)
    * `Nixu_x.x.x_x64.dmg` (Mac Intel 芯片安装包)
    * `Nixu_x.x.x_aarch64.dmg` (Mac M 芯片安装包)

点击即可下载至本地进行测试和分发！所有跨平台环境困扰彻底解除。
