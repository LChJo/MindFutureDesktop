# devtools 测试规格

> 由 tdd_gen 生成。

## 概述

- 覆盖率目标: 80%
- 测试框架: 待确认
- 测试类型: unit, integration

## 可测接口

- **getSessionMetrics** `[inbound]`
  - Get metrics for a Cowork session

## 测试用例

### getSessionMetrics

- [ ] 正常路径: Get metrics for a Cowork session
- [ ] 推断错误: Invalid input: validation failure
- [ ] 推断错误: Persisted data malformed or missing
- [ ] 推断错误: Unexpected failure / runtime exception
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation


## 骨架文件

→ [tests/devtools.test.ts](../../tests/devtools.test.ts)
