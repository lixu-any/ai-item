---
name: tauri-window-permissions
description: 每次创建新的 Tauri 独立窗口时，必须同步更新 capabilities/default.json 中的 windows 列表
---

# 关键规则：Tauri 窗口权限登记

## 背景

Tauri 2.x 的 capability 系统使用白名单机制：只有在 `windows` 数组中明确列出的窗口 label，才被允许调用 `permissions` 中声明的命令。 

**如果你通过 `new WebviewWindow('my-window', {...})` 创建了一个新窗口**，该窗口默认不会继承任何权限，调用 `invoke(...)` 时会报如下错误：

```
save_setting not allowed on window "settings", webview "settings", URL: local
allowed on: [windows: "main", "add-host", ...]
```

## 强制执行规则

每当你用 `new WebviewWindow(label, {...})` 创建一个新的独立窗口时，**必须同时**修改以下文件：

### 文件：`src-tauri/capabilities/default.json`

在 `"windows"` 数组中添加该窗口的 label（支持通配符 `*`）：

```json
"windows": [
  "main",
  "add-host",
  "edit-host-*",
  "settings",
  "your-new-window"
]
```

## 常见易忘窗口

| 窗口用途 | Label 示例 | 说明 |
|------|------|------|
| 全局设置 | `settings` | 新增时已写入 |
| 添加主机 | `add-host` | 已写入 |
| 编辑主机 | `edit-host-*` | 已写入，使用通配符 |
| 添加分组 | `add-group` | 已写入 |
| 编辑分组 | `edit-group-*` | 已写入，使用通配符 |

## AI 操作 Checklist

当你为用户创建一个功能需要新开窗口时（在 `openXxxWindow()` 函数里 new WebviewWindow），你必须：

1. ✅ 创建 `WebviewWindow` 时指定唯一 `label`
2. ✅ 立刻同步修改 `src-tauri/capabilities/default.json` 的 `windows` 数组，将该 label 加入
3. ✅ 在 `App.vue` 的 `onMounted` 中处理 `view=xxx` 参数分支，正确渲染独立窗口内容
