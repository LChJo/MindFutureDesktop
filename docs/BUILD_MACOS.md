# macOS Build Guide

本文档描述如何为 macOS 构建 Moonton Claude Desktop。

## 系统要求

- macOS 11.0 (Big Sur) 或更高版本
- Xcode Command Line Tools
- Node.js 20+
- Rust 1.77.2+

## 环境配置

### 1. 安装 Xcode Command Line Tools

```bash
xcode-select --install
```

### 2. 安装 Rust 和 macOS 目标

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 添加 macOS 构建目标
rustup target add x86_64-apple-darwin aarch64-apple-darwin
```

### 3. 安装 Node.js

```bash
# 使用 Homebrew
brew install node

# 或从官网下载
# https://nodejs.org/
```

## 构建步骤

### 开发构建（无签名）

1. **安装依赖**

```bash
npm install
```

2. **准备 Node.js Universal Binary**

```bash
npm run prepare:nodejs-macos
```

这会下载并创建包含 Intel x64 和 Apple Silicon ARM64 的通用二进制文件。

3. **构建前端**

```bash
npm run build:frontend
```

4. **构建应用**

```bash
# Universal binary (推荐 - 支持所有 Mac)
npm run build:mac

# 或只构建特定架构
npm run build:mac-intel  # 仅 Intel x64
npm run build:mac-arm    # 仅 Apple Silicon ARM64
```

5. **运行应用**

```bash
# 应用位置
open "src-tauri/target/universal-apple-darwin/release/bundle/macos/Moonton Claude Desktop.app"
```

**注意**: 首次运行未签名的应用时，需要右键点击应用 > 打开，以绕过 Gatekeeper 警告。

### 生产构建（签名和公证）

生产构建需要 Apple Developer 账户（$99/年）。

#### 前置要求

1. **Apple Developer ID 证书**
   - 从 Apple Developer 网站下载开发者 ID 应用证书
   - 安装到钥匙串

2. **App-specific Password**
   - 访问 https://appleid.apple.com
   - 生成 App-specific password 用于公证

#### 手动签名流程

1. **构建未签名的应用**

```bash
npm run build:mac
```

2. **签名 .app bundle**

```bash
codesign --force --deep --sign "Developer ID Application: Your Name (TEAM_ID)" \
  --options runtime --timestamp \
  --entitlements src-tauri/entitlements.plist \
  "src-tauri/target/universal-apple-darwin/release/bundle/macos/Moonton Claude Desktop.app"
```

3. **创建 DMG**

```bash
hdiutil create -volname "Moonton Claude Desktop" \
  -srcfolder "src-tauri/target/universal-apple-darwin/release/bundle/macos/Moonton Claude Desktop.app" \
  -ov -format UDZO \
  "Moonton-Claude-Desktop-macos-universal.dmg"
```

4. **签名 DMG**

```bash
codesign --force --sign "Developer ID Application: Your Name (TEAM_ID)" \
  --timestamp \
  "Moonton-Claude-Desktop-macos-universal.dmg"
```

5. **公证应用**

```bash
xcrun notarytool submit "Moonton-Claude-Desktop-macos-universal.dmg" \
  --apple-id "your@email.com" \
  --password "app-specific-password" \
  --team-id "TEAM_ID" \
  --wait
```

6. **装订公证票据**

```bash
xcrun stapler staple "Moonton-Claude-Desktop-macos-universal.dmg"
```

#### 自动化构建（GitHub Actions）

推送到 `mac` 或 `main` 分支会自动触发构建。推送 tag（如 `v1.2.0`）会触发签名和公证。

**需要配置的 GitHub Secrets**:
- `MACOS_CERTIFICATE` - Base64 编码的 .p12 证书
- `MACOS_CERTIFICATE_PWD` - 证书密码
- `APPLE_DEVELOPER_ID` - 开发者 ID（格式: "Developer ID Application: Name (TEAM_ID)"）
- `APPLE_ID` - Apple ID 邮箱
- `APPLE_APP_SPECIFIC_PASSWORD` - App-specific password
- `APPLE_TEAM_ID` - 10 位字符的 Team ID
- `KEYCHAIN_PWD` - 临时钥匙串密码

## 验证构建

### 检查 Universal Binary

```bash
cd src-tauri/target/universal-apple-darwin/release/bundle/macos
lipo -info "Moonton Claude Desktop.app/Contents/MacOS/moonton-claude-desktop"

# 应该显示: Architectures in the fat file: ... are: x86_64 arm64
```

### 检查代码签名

```bash
codesign -dvvv "Moonton Claude Desktop.app"
spctl -a -vv "Moonton Claude Desktop.app"
```

### 验证公证

```bash
spctl -a -vv "Moonton-Claude-Desktop-macos-universal.dmg"
stapler validate "Moonton-Claude-Desktop-macos-universal.dmg"
```

## 故障排查

### "Moonton Claude Desktop.app 已损坏"

```bash
xattr -cr "/Applications/Moonton Claude Desktop.app"
```

### Git 未找到

用户需要安装 Git：

```bash
# 方法 1: Xcode Command Line Tools
xcode-select --install

# 方法 2: Homebrew
brew install git
```

### Node.js 未检测到

应用会使用内置的 Node.js。如果有问题：
1. 检查 `~/Library/Application Support/MoontonClaudeDesktop/nodejs/` 目录
2. 确认 `node` 文件有执行权限: `chmod +x ~/Library/Application\ Support/MoontonClaudeDesktop/nodejs/node`

### 终端无法打开

检查权限设置:
- 系统设置 > 隐私与安全 > 完全磁盘访问
- 添加 Moonton Claude Desktop 到允许列表

## 开发技巧

### 快速开发迭代

```bash
# 开发模式（不需要 Node.js universal binary）
npm run dev

# 只构建前端
npm run build:frontend
```

### 调试

```bash
# 启用 Rust backtrace
RUST_BACKTRACE=1 open "Moonton Claude Desktop.app"

# 查看日志
tail -f ~/Library/Logs/MoontonClaudeDesktop/moonton-claude.log
```

### 测试不同架构

在 Apple Silicon Mac 上测试 Intel 版本:
```bash
# 构建 Intel 版本
npm run build:mac-intel

# 使用 Rosetta 2 运行
arch -x86_64 open "src-tauri/target/x86_64-apple-darwin/release/bundle/macos/Moonton Claude Desktop.app"
```

## 相关文档

- [Tauri 文档](https://tauri.app/)
- [Apple 代码签名指南](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)

## 支持

如遇问题，请查看:
1. GitHub Issues
2. 构建日志 (GitHub Actions)
3. 本地日志文件
