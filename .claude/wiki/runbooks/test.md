# Test 手册

> 由 wiki_gen 生成。

## 命令

```bash
npm run test
```
```bash
npm run test:coverage
```
```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## 前置条件

无

## 故障排查

- 依赖未安装 → 运行 `npm install`
- 缓存问题 → 删除 `node_modules` 后重新安装
- 模块缓存问题 → 运行 `go clean -modcache`
- 依赖下载失败 → 检查 GOPROXY 环境变量，或设置 `GOFLAGS=-mod=mod`
- 构建缓存损坏 → 运行 `cargo clean`
- 链接器错误 → 检查系统 C 工具链（gcc/clang/msvc）是否已安装
- 检查环境变量是否配置正确（参见 `.env.example`）
- 查看完整错误日志以定位具体失败原因
