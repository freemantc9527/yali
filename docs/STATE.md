# STATE · 项目状态接力（跨 session 冷启动先读本文件）

> 更新规则：每张任务卡完成时更新；本文件是 Agent 的"记忆外置盘"。最后更新：2026-11-17（迁移后体检 + T-005 脚手架）。

## 当前阶段：M-1 收尾 → M0 内核

- [x] GitHub 仓库 `freemantc9527/yali`（public，Apache-2.0）
- [x] README / AGENTS.md / .gitignore / Brewfile
- [x] 提交身份：noreply 邮箱 + gh 凭证助手
- [x] Cargo workspace 骨架（yali-core / yali-cli）
- [x] **CLT-only 终验通过（2026-11-17 10:27）**：`cargo build -p yali-app` Finished；`cargo test` 3 passed——ADR-002 核心假设成立，无需触发 ADR-003 降级
- [x] src-tauri 占位图标（generate_context! 编译期依赖 icons/icon.png）
- [x] **ADR 入库（2026-11-17）**：`docs/decisions/` ADR-001（废弃）/ ADR-002（接受）/ ADR-003（兜底备用）/ ADR-004（pnpm）
- [x] **ui/ 最小 Vite 脚手架（T-005 部分完成）**：Vite 8 + React 19 + TS + vitest；`pnpm --dir ui test` 4 passed；`pnpm --dir ui build` → `ui/dist`
- [x] **release 构建坑修复**：`frontendDist` 缺失会让非 dev 构建 panic → `src-tauri/build.rs` 在 codegen 前兜占位页
- [x] docs/ 自包含规格迁移（DESIGN/BENCHMARKS/TOOLCHAIN/NAMING/WORKFLOW/CROSSPLATFORM）
- [ ] `ui/` 与内核接线：Profile 表单 + 拖入队列（T-006 起，按任务卡）
- [ ] benchkit/ 合成金样素材生成脚本（T-004）
- [ ] `docs/tasks/` 任务卡目录（随 T-002 落盘）

## 关键决策记录（摘要，全文在 docs/decisions/）
- 技术栈：Tauri 2 + Rust + React；**硬约束无 Xcode**（ADR-002）
- 前端包管理器：**pnpm**，命令一律 `pnpm --dir ui …`（ADR-004）；锁文件入库，`node_modules/` 不入库
- 右键主通道：用户级 `.workflow`（免 Xcode 免签名）；NSServices 为 M2 可选
- 命名：鸭梨/Yali，全部产出物名称见 docs/NAMING.md
- 分发：本地工具定位，证书延后；brew tap 源码构建为主渠道
- 平台：macOS 先行，Linux→Windows 备忘见 docs/CROSSPLATFORM.md

## 下一步（按序）
> **执行计划已落卡**：冷启动先读 `docs/plans/PLAN-M0.md`（临时文档，M0 完成后移除），卡片在 `docs/tasks/`。

1. T-002: Profile Schema + Store + 五档 fixtures（`docs/tasks/T-002-profile-schema.md`）
2. T-003: EngineRegistry + 子进程池 + CLI 真实现（T-003）
3. T-004: benchkit 合成金样 + bench.sh（T-004）
4. T-005 收尾: UI 接线（T-005，manual-check）

## 坑账本
- `tauri::generate_context!` **编译期**要求 `src-tauri/icons/icon.png` 存在（即使 bundle.active=false）
- **debug 与 release 对 `frontendDist` 的校验不同**：debug 因 `dev_url` 有值直接短路，release/codegen 会 panic「frontendDist doesn't exist」。`ui/dist` 是构建产物不入库 → 已由 `src-tauri/build.rs` 兜占位页；验证仓库可复现性时必须跑 `--release`
- gh 2.104 无 `-g` 短参；`gh repo create` 远端新建用 `--add-readme --license --clone` 组合
- 迁移后 `target/` 缓存带着旧绝对路径，指纹脏；换路径后建议 `cargo clean` 重建一次
