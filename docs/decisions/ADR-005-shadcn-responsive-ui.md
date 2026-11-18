# ADR-005 · GUI 采用 shadcn/ui 与 Tailwind CSS 4

> 接受，2026-11-18。用户指定 shadcn/ui 并确认效果图与响应式方向，Agent 细化方案。T-021 已实施，待原生人工验收。

## 背景与取舍

原 GUI 按卡片逐步接线，缺少完整的批处理流程。采用 shadcn/ui 的 Radix 控件源码，复用菜单、对话框、滑杆的交互和可访问性基础，替代 CSS Modules 与自建通用控件。组件入库后由项目维护，不混用其他控件基础。

## 决策与边界

保留 React / TypeScript / Vite / Tauri / Rust 和 pnpm，只增 Tailwind CSS 4、CSS 语义变量与 Lucide。工作台用 reducer 和受控数据；不加全局状态、数据获取或表单框架。原生多选、目录选择与 Finder 定位用官方 dialog / opener，只授予所需能力。

继续用既有内核、行为与 Schema，本批草稿通过新 command 传递。Tailwind 4 的 Safari 16.4+ 基础要求低于项目的 Safari 17 构建目标；实际 WKWebView 仍需人工验收。

若组件基础出现无法解决的 WebView 兼容问题，另立 ADR 复评；回退范围仅限本卡前端 / 壳层接线与依赖，不迁移或删除用户数据。

工作台合同见 [UI-REDESIGN](../UI-REDESIGN.md)，实际结果及剩余验收见 [T-021](../tasks/T-021-responsive-workbench.md)。
