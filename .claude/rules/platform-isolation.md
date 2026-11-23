# ARCH-PLATFORM-001 把平台代码隔离在桥接层后面

## 规则

UI 组件与业务逻辑不得直接使用 `localStorage` / `sessionStorage` / `document` / `window.navigator` 等浏览器 API；通过 `hooks/` 或 `store/` 层的适配器调用。

```text
错误做法：
components/Card -> window.localStorage.setItem("cards", ...)

正确做法：
components/Card -> useBoardStore().saveCard(...)   // store 内封装 localStorage 与配额降级
```

原因：浏览器 API 是副作用来源；经过 store/hook 适配器后，组件可测试、持久化策略可替换（localStorage / IndexedDB / 内存）。
适用范围：所有前端组件与业务逻辑层。
