#!/bin/bash
set -e

APP_NAME="Nixu"

echo "==================================="
echo "🚀 开始构建 $APP_NAME Mac 原生版本..."
echo "==================================="

# 执行 Tauri 的本地默认硬件架构构建
npm run tauri build

# 构建完成后的 App 包默认输出路径
APP_PATH="src-tauri/target/release/bundle/macos/${APP_NAME}.app"

if [ ! -d "$APP_PATH" ]; then
    echo "❌ 构建中断或失败：未找到生成的 App 捆绑包位于 ($APP_PATH)"
    exit 1
fi

echo "✅ 编译成功！准备为您安全安装到底层应用库..."

# 如果应用当前正在后台运行，则平滑结束进程以防止文件占用
if pgrep -x "$APP_NAME" > /dev/null; then
    echo "⚠️ 发现正在运行的旧版本，正在关闭以保障覆盖安装..."
    pkill -x "$APP_NAME" || true
    sleep 1.5
fi

# 开始覆盖
if [ -d "/Applications/${APP_NAME}.app" ]; then
    echo "🗑 正在卸载旧版本残留文件..."
    rm -rf "/Applications/${APP_NAME}.app"
fi

echo "📦 正在无缝迁移全新版本至 /Applications 目录..."
cp -R "$APP_PATH" /Applications/

echo "🎉 一键安装大功告成！正在为您唤醒应用..."
open -a "$APP_NAME"

echo "==================================="
echo "✨ Nixu 终端已就绪，尽情享受起飞的快感！"
echo "==================================="
