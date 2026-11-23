# Moonton Claude Desktop 自动更新配置指南

## 概述

Moonton Claude Desktop 内置自动更新机制，通过检查远程 JSON 清单文件来判断是否有新版本可用。更新流程：

1. 应用启动时静默检查更新（失败不影响启动）
2. 用户可在 **设置 > 关于** 手动检查更新
3. 发现新版本后弹窗确认，下载安装包并启动安装程序
4. 安装程序启动后应用自动退出

## 服务端配置

### 目录结构

在 HTTPS 静态文件服务器上部署以下文件：

```
https://example.com/
├── update-manifest.json              # 更新清单（必需）
└── releases/
    └── 1.3.0/                        # 按版本号组织
        ├── Moonton-Claude-Desktop_1.3.0_x64-setup.exe    # Windows 安装包
        └── Moonton-Claude-Desktop_1.3.0_universal.dmg    # macOS 安装包
```

### 清单文件格式

`update-manifest.json` 内容如下：

```json
{
  "version": "1.3.0",
  "releaseDate": "2026-02-10",
  "releaseNotes": "修复已知问题，提升性能表现",
  "platforms": {
    "windows-x86_64": {
      "url": "https://example.com/releases/1.3.0/Moonton-Claude-Desktop_1.3.0_x64-setup.exe",
      "size": 85000000
    },
    "darwin-universal": {
      "url": "https://example.com/releases/1.3.0/Moonton-Claude-Desktop_1.3.0_universal.dmg",
      "size": 92000000
    }
  }
}
```

### 字段说明

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `version` | string | 是 | 最新版本号（语义化版本，如 `1.3.0`） |
| `releaseDate` | string | 否 | 发布日期 |
| `releaseNotes` | string | 否 | 更新说明，会显示在用户确认弹窗中 |
| `platforms` | object | 是 | 各平台安装包信息 |

**平台键名：**

| 键名 | 平台 |
|------|------|
| `windows-x86_64` | Windows x64 |
| `darwin-universal` | macOS Universal Binary |

**平台资产字段：**

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `url` | string | 是 | 安装包下载地址（必须 HTTPS） |
| `size` | number | 否 | 文件大小（字节），用于显示下载进度和大小提示 |

## 安全要求

- 清单 URL 和下载 URL **必须使用 HTTPS 且证书须受系统信任**
- 下载的安装包文件名会被自动清理，防止路径穿越攻击
- 安装程序只允许特定扩展名：
  - Windows: `.exe`, `.msi`
  - macOS: `.dmg`, `.pkg`
- 安装程序必须位于应用缓存目录内，外部路径会被拒绝

## 客户端配置

### 更新地址优先级

应用按以下优先级确定更新清单地址：

| 优先级 | 来源 | 位置 | 说明 |
|--------|------|------|------|
| 1 (最高) | 用户设置 | 设置 > 关于 > 更新源 | 用户手动填写，保存在 `settings.json` |
| 2 | 外部配置文件 | `update-config.json` | 运维/部署脚本管理，**无需重新编译** |

公开版本需要配置更新源才能检查更新。

### 外部配置文件（推荐）

在以下路径创建 `update-config.json` 即可设置更新地址，无需重新编译应用：

- Windows 和 macOS：`~/.MoontonClaudeDesktop/config/update-config.json`

文件格式：

```json
{
  "updateUrl": "https://example.com/update-manifest.json"
}
```

变更地址时只需修改此文件，下次应用启动或手动检查更新时自动生效。

**适用场景：**

- 需要经常变更更新源地址
- 企业内部部署，通过脚本统一下发配置
- 测试环境指向测试服务器

### 用户手动设置

在 **设置 > 关于** 页面底部的「更新源」区域填入自定义 URL。此设置优先级最高，会覆盖外部配置文件。留空则回退到外部配置文件。

### 手动检查更新

打开 **设置 > 关于**，点击「检查更新」按钮。

## 发布新版本流程

1. 更新 `package.json` 中的 `version` 字段（如 `1.3.0`）
2. 执行构建命令生成安装包：
   ```bash
   npm run build:windows    # Windows
   npm run build:mac        # macOS
   ```
3. 将安装包上传到服务器对应版本目录
4. 更新 `update-manifest.json`，修改 `version`、`releaseNotes` 和各平台的 `url`、`size`
5. 验证：用旧版本客户端检查更新，确认能正确检测到新版本并完成下载安装

## 故障排除

| 问题 | 原因 | 解决方法 |
|------|------|----------|
| 启动时无更新提示 | 服务器不可达或版本相同 | 正常行为，启动时静默失败不报错 |
| 手动检查报错 | 服务器不可达或清单格式错误 | 检查 URL 是否可访问，清单 JSON 格式是否正确 |
| 检测到更新但无下载链接 | 清单中缺少当前平台的资产 | 在 `platforms` 中添加对应平台的条目 |
| 下载失败 | 网络中断或下载 URL 无效 | 检查下载 URL 是否可访问，重新点击检查更新 |
| HTTPS 错误 | 使用了 HTTP 地址 | 清单 URL 和下载 URL 均须为 HTTPS |

## 技术实现

- 后端：`src-tauri/src/updater.rs`（Rust，4 个 Tauri 命令）
- 前端：`src/composables/useAutoUpdate.ts`（Vue 组合式函数）
- 启动检查：`src/App.vue`（`onMounted` 中静默调用）
- 设置界面：`src/components/Settings.vue`（关于标签页）
- 下载缓存：`%LOCALAPPDATA%/MoontonClaudeDesktop/cache/`
