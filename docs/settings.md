# 设置功能说明

设置面板提供桌面外观、Node.js 环境、Claude Code 管理、环境变量和快捷键的配置功能。

## 首次启动引导

首次启动应用时，如果检测到 Claude Code 未安装，会显示全屏引导界面：

1. **自动检测环境** - 启动时自动检测 Node.js 和 Claude Code 安装状态
2. **一键安装** - 点击「安装 Claude Code」按钮即可完成安装
3. **进度显示** - 实时显示 Node.js 释放和 Claude Code 安装进度
4. **自动进入桌面** - 安装完成后自动关闭引导界面，进入正常桌面

> 已安装 Claude Code 的用户会直接进入桌面，不会看到引导界面。

## 功能概述

设置面板包含五个主要标签页：

### 1. 外观设置

自定义桌面壁纸。

**功能特性：**
- 预览当前壁纸
- 选择本地图片作为壁纸（支持 PNG、JPG、JPEG、GIF、WEBP、BMP）
- 恢复默认壁纸

**使用方法：**
1. 点击「选择图片」按钮
2. 在文件选择器中选择图片文件
3. 图片将自动应用为桌面背景（拉伸显示）

### 2. Node.js 环境

管理内嵌的 Node.js 运行环境。

MindFuture Desktop 内嵌了 Node.js v20.x LTS 便携版，用于支持 Claude Code CLI 的运行。

**状态信息：**
- 安装状态：显示 Node.js 是否已释放到本地
- 版本号：当前安装的 Node.js 版本
- 路径：Node.js 安装位置（`%LOCALAPPDATA%\MindFuture\nodejs`）

**操作按钮：**
- **安装 Node.js**：首次使用时，点击释放内嵌的 Node.js 到本地
- **重新安装**：如果出现问题，可重新释放 Node.js
- **刷新状态**：刷新当前状态显示

**注意事项：**
- 首次启动应用时会自动释放 Node.js（后台静默执行）
- 内嵌的 Node.js 路径会自动注入到终端的 PATH 环境变量

### 3. Claude Code

管理 Claude Code CLI 的安装和更新。

**状态信息：**
- 安装状态：显示 Claude Code 是否已安装
- 版本号：当前安装的 Claude Code 版本

**操作按钮：**
- **安装 Claude Code**：通过 npm 全局安装 @anthropic-ai/claude-code
- **更新 Claude Code**：更新到最新版本

**常用命令：**
- `claude` - 启动 Claude Code 交互模式
- `claude -c` - 继续上次对话
- `claude /init` - 初始化项目配置
- `claude /review` - 代码审查

**注意事项：**
- 需要先安装 Node.js 才能安装 Claude Code
- Claude Code 安装需要网络连接
- 安装完成后，可在终端中直接使用 `claude` 命令

### 4. 环境变量

配置终端启动时的环境变量，分为四个板块：

#### Claude 环境设置

用于配置 Claude Code CLI 的环境变量：

| 变量名 | 说明 | 示例 |
|--------|------|------|
| `ANTHROPIC_AUTH_TOKEN` | API 认证令牌 | `sk-ant-xxxxx` |
| `ANTHROPIC_MODEL` | 使用的模型名称 | `claude-sonnet-4-20250514` |
| `ANTHROPIC_BASE_URL` | API 基础 URL | `https://api.anthropic.com` |

#### Gemini 环境设置

用于配置 Google Gemini 的环境变量：

| 变量名 | 说明 | 示例 |
|--------|------|------|
| `GOOGLE_GEMINI_BASE_URL` | Gemini API 基础 URL | `https://generativelanguage.googleapis.com` |
| `GEMINI_API_KEY` | Gemini API 密钥 | `AIzaSyxxxxx` |

#### Codex 环境设置

用于配置 OpenAI Codex 的环境变量：

| 变量名 | 说明 | 示例 |
|--------|------|------|
| `OPENAI_API_KEY` | OpenAI API 密钥 | `sk-xxxxx` |

#### 自定义环境变量

添加任意自定义环境变量。

**使用方法：**
1. 在「变量名」输入框中输入环境变量名称
2. 在「变量值」输入框中输入对应的值
3. 点击「添加」按钮
4. 要删除变量，点击对应行的删除按钮

**注意事项：**
- 留空的预设变量不会被设置
- 环境变量会覆盖 PowerShell profile 中的同名设置
- 仅对新打开的终端生效，已打开的终端不受影响

### 5. 快捷键

显示当前可用的快捷键列表：

| 快捷键 | 功能 |
|--------|------|
| 双击 | 打开图标/恢复窗口 |
| 右键单击 | 打开右键菜单 |
| 拖拽 | 移动图标/窗口 |
| `Ctrl + C` | 复制终端选中文本 |
| `Ctrl + V` | 粘贴到终端 |
| `Ctrl + Shift + C` | 复制（备选） |
| `Ctrl + Shift + V` | 粘贴（备选） |

## 数据存储

设置数据存储在本地，位置为应用数据目录下的配置文件中。点击「保存设置」按钮可手动保存当前配置。

## 使用场景

### 配置 Claude Code 代理

如果需要通过代理访问 Claude API：

1. 打开设置 → 环境变量
2. 在 Claude 环境设置中填写：
   - `ANTHROPIC_BASE_URL`: 代理服务器地址
   - `ANTHROPIC_AUTH_TOKEN`: API 令牌
3. 打开新终端，运行 `claude` 即可使用代理

### 切换 Claude 模型

1. 打开设置 → 环境变量
2. 在 `ANTHROPIC_MODEL` 中填写目标模型名称
3. 打开新终端生效
