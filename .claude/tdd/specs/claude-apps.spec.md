# claude-apps 测试规格

> 由 tdd_gen 生成。

## 概述

- 覆盖率目标: 80%
- 测试框架: 待确认
- 测试类型: unit, integration

## 可测接口

- **df_list_functions** `[inbound]`
  - List all installed dynamic functions
- **df_create_function** `[inbound]`
  - Create a new dynamic function from template
- **df_start_backend** `[inbound]`
  - Start the backend process for a dynamic function
- **df_stop_backend** `[inbound]`
  - Stop the backend process
- **mcd.exec** `[inbound]`
  - Execute shell command from within a dynamic function (sandboxed)
- **mcd.kill** `[inbound]`
  - Kill a running process started by mcd.exec
- **mcd.openExternal** `[inbound]`
  - Open URL or file with system default handler

## 测试用例

### df_list_functions

- [ ] 正常路径: List all installed dynamic functions
- [ ] 推断错误: Invalid input: validation failure
- [ ] 推断错误: Unexpected failure / runtime exception
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation

### df_create_function

- [ ] 正常路径: Create a new dynamic function from template
- [ ] 推断错误: Invalid input: validation failure
- [ ] 推断错误: Concurrent state mutation conflict
- [ ] 推断错误: Unexpected failure / runtime exception
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation

### df_start_backend

- [ ] 正常路径: Start the backend process for a dynamic function
- [ ] 推断错误: Invalid input: validation failure
- [ ] 推断错误: Unexpected failure / runtime exception
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation

### df_stop_backend

- [ ] 正常路径: Stop the backend process
- [ ] 推断错误: Invalid input: validation failure
- [ ] 推断错误: Unexpected failure / runtime exception
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation

### mcd.exec

- [ ] 正常路径: Execute shell command from within a dynamic function (sandboxed)
- [ ] 推断错误: Invalid input: validation failure
- [ ] 推断错误: Unexpected failure / runtime exception
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation

### mcd.kill

- [ ] 正常路径: Kill a running process started by mcd.exec
- [ ] 推断错误: Invalid input: validation failure
- [ ] 推断错误: Unexpected failure / runtime exception
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation

### mcd.openExternal

- [ ] 正常路径: Open URL or file with system default handler
- [ ] 推断错误: Invalid input: validation failure
- [ ] 推断错误: Unexpected failure / runtime exception
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation
- [ ] 边界条件: Empty/null input values
- [ ] 边界条件: User cancels mid-operation


## 骨架文件

→ [tests/claude-apps.test.ts](../../tests/claude-apps.test.ts)
