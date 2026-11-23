# coworks 测试规格

> 由 tdd_gen 生成。

## 概述

- 覆盖率目标: 80%
- 测试框架: 待确认
- 测试类型: unit, integration

## 可测接口

- **startSession** `[inbound]`
  - Start a new Cowork session
- **sendMessage** `[inbound]`
  - Send user message to Claude
- **sidecar_spawn** `[outbound]`
  - Spawn Claude Agent SDK sidecar process

## 测试用例

### startSession

- [ ] 正常路径: Start a new Cowork session
- [ ] 推断错误: Invalid input: validation failure
- [ ] 推断错误: Unexpected failure / runtime exception
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation

### sendMessage

- [ ] 正常路径: Send user message to Claude
- [ ] 推断错误: Invalid input: validation failure
- [ ] 推断错误: Unexpected failure / runtime exception
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation

### sidecar_spawn

- [ ] 正常路径: Spawn Claude Agent SDK sidecar process
- [ ] 推断错误: Invalid input: validation failure
- [ ] 推断错误: Unexpected failure / runtime exception
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation


## 骨架文件

→ [tests/coworks.test.ts](../../tests/coworks.test.ts)
