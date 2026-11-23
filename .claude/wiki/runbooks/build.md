# Build 手册

> 由 wiki_gen 生成。

## 命令

```bash
npm run build:windows
```
```bash
npm run build:mac
```

## 前置条件

无

## 故障排查

- 依赖未安装 → 运行 `npm install`
- 缓存问题 → 删除 `node_modules` 后重新安装
- 构建缓存损坏 → 运行 `cargo clean`
- 链接器错误 → 检查系统 C 工具链（gcc/clang/msvc）是否已安装
- 检查环境变量是否配置正确（参见 `.env.example`）
- 查看完整错误日志以定位具体失败原因
