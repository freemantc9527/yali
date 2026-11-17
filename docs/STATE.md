# STATE · 项目状态接力（跨 session 冷启动先读本文件）

> 更新规则：每张任务卡完成时更新；本文件是 Agent 的"记忆外置盘"。最后更新：2026-11-17 M-1。

## 当前阶段：M-1 整备

- [x] GitHub 仓库 `freemantc9527/yali`（public，Apache-2.0）
- [x] README / AGENTS.md / .gitignore / Brewfile
- [x] 提交身份：noreply 邮箱 + gh 凭证助手
- [x] Cargo workspace 骨架（yali-core / yali-cli）
- [x] **CLT-only 终验通过（2026-11-17 10:27）**：`cargo build -p yali-app` Finished；`cargo test` 3 passed——ADR-002 核心假设成立，无需触发 ADR-003 降级
- [x] src-tauri 占位图标（生成期发现 generate_context! 编译期依赖 icons/icon.png，已入坑账本）
- [ ] ui/ Vite+React 正式脚手架（T-005）
- [ ] benchkit/ 合成金样素材生成脚本（T-004）
- [x] docs/ 自包含规格迁移（DESIGN/BENCHMARKS/TOOLCHAIN/NAMING/WORKFLOW/CROSSPLATFORM）

## 关键决策记录（摘要）
- 技术栈：Tauri 2 + Rust + React；**硬约束无 Xcode**（实证边界见 docs/TOOLCHAIN.md）
- 右键主通道：用户级 `.workflow`（免 Xcode 免签名）；NSServices 为 M2 可选
- 命名：鸭梨/Yali，全部产出物名称见 docs/NAMING.md
- 分发：本地工具定位，证书延后；brew tap 源码构建为主渠道
- 平台：macOS 先行，Linux→Windows 备忘见 docs/CROSSPLATFORM.md

## 下一步（按序）
1. T-002: profile.schema.json + fixtures 入库
2. T-003: yali-core 骨架（EngineRegistry 接口 + 子进程池）
3. T-004: benchkit 合成金样 + bench.sh
4. T-005: ui/ Vite+React 脚手架

## 坑账本
- `tauri::generate_context!` **编译期**要求 `src-tauri/icons/icon.png` 存在（即使 bundle.active=false）——新克隆仓库需先放占位图标
- gh 2.104 无 `-g` 短参；`gh repo create` 远端新建用 `--add-readme --license --clone` 组合
