# ADR-004 · 前端包管理器：pnpm

> 状态：**接受** · 日期 2026-11-17 · 决策人：用户

## 背景

UI 需要 Vite + React 脚手架（T-005）。npm 在本地开发机上存在两个实际问题：
依赖树扁平化导致幽灵依赖、以及多项目重复占用磁盘（本项目未来还有 playwright 等重依赖）。

## 决策

前端一律使用 **pnpm**，不再出现 `npm` / `npx` 命令。

- `ui/package.json` 的包名 `yali-ui`（私有，不发布）
- 构建/开发命令统一经 `--dir`：`pnpm --dir ui install` / `pnpm --dir ui build` / `pnpm --dir ui test`
- Tauri 的 `beforeDevCommand` / `beforeBuildCommand` 同步改为 pnpm（见 `src-tauri/tauri.conf.json`）
- 锁文件 `ui/pnpm-lock.yaml` **入库**（可复现构建），`node_modules/` 不入库
- 文档里出现 `npx` 的场景统一写成 `pnpm exec`（如 `pnpm exec playwright test`）

## 后果

- 放弃：npm/npx 的命令习惯（含 README、AGENTS、CONTRIBUTING 的示例）
- 承担：新环境多一个前置依赖 `pnpm`（Brewfile 已注明，CLT-only 约束不受影响）
- 回滚条件：pnpm 与 CI runner 出现不可解决的兼容问题 → 重新评估，另立 ADR
