# PLAN · M0 执行计划（临时文档，执行完归档或删除）

> **用途**：给新 session 的冷启动执行单。本文件是临时的——四张卡全部落地后，把本文删除或移入 `docs/decisions/` 归档，进度以 `docs/STATE.md` 为准。
> 生成时间：2026-11-17 · 前置状态：M-1 完成 + 迁移体检通过（commits 99b1807 → d736675）

## 0. 冷启动清单（新 session 第一件事）

```bash
cd /Users/taocheng/career/yali
git pull origin main && git log --oneline -3   # 应看到 d736675
cargo test                                     # 应 3 passed，零警告
pnpm --dir ui test                             # 应 4 passed
```

然后按序读：`docs/STATE.md`（进度）→ 本文件（本轮计划）→ 对应任务卡。

## 1. 硬约束（每次动手前重读，违者返工）

1. **无 Xcode**：禁止 `swift test` / `xcodebuild` / SwiftUI 宏（`docs/TOOLCHAIN.md` §2 实证边界）
2. **pnpm**：前端命令一律 `pnpm --dir ui …`，禁 npm/npx（ADR-004）
3. **命名**：以 `docs/NAMING.md` 为准；`crates/yali-integration` 是预注册名，本轮**不创建**
4. **依赖白名单**：serde / serde_json / anyhow / clap（tokio 待 T-003 按需引回）；新增依赖先在任务卡里写理由
5. **一卡一 commit**：验收命令的输出摘要写进 commit message；每卡完成即更新 `docs/STATE.md` 并 push
6. 16GB 机器：构建 `cargo build -j4`；不跑无意义的全量 release 构建（`--release` 全依赖图约 1 分钟，需要时再跑）

## 2. 执行顺序与依赖

```
T-002 (schema+fixtures+Store) ──► T-003 (EngineRegistry+CLI 真实现) ──► T-004 (benchkit 金样) ──► T-005 (UI 接线)
        契约先行，零 UI 依赖                核心逻辑                    质量守门                    收尾接线
```

- T-002 完成前不许写任何压缩实现（契约先行）
- T-003 的引擎参数映射以 `docs/DESIGN.md` §3 为准，实测数据在 `docs/BENCHMARKS.md`
- T-004 是 T-003 质量变更的守门工具，先于 T-005
- T-005 只做接线，不做视觉迭代（AGENTS.md 硬性规则）

## 3. 卡片索引

| 卡 | 文件 | 预估 | 关键验收 |
|---|---|---|---|
| T-002 | [docs/tasks/T-002-profile-schema.md](../tasks/T-002-profile-schema.md) | 2h | `cargo test -p yali-core profile` 绿；五档 fixtures 与 schema 一致 |
| T-003 | [docs/tasks/T-003-engine-registry.md](../tasks/T-003-engine-registry.md) | 4h | `yali compress` 单图冒烟；负优化拦截；undo 清单可还原 |
| T-004 | [docs/tasks/T-004-benchkit.md](../tasks/T-004-benchkit.md) | 2h | 合成金样生成 + bench.sh 三指标出表 + 基线入库 |
| T-005 | [docs/tasks/T-005-ui-wiring.md](../tasks/T-005-ui-wiring.md) | 3h | 拖入 jpg → 产出 `-{profile}` 文件（manual-check 真机） |

## 4. M0 出口条件（全部满足才算 M0 完成）

- [ ] 合成集回归达标（bench.sh 三指标，SSIM 门：level≥5 时 ≥0.95）
- [ ] 单图 p50 < 1.5s（DESIGN §8）
- [ ] undo 100% 还原
- [ ] `cargo test` 全绿 + `pnpm --dir ui test` 全绿
- [ ] STATE.md 更新为「M0 完成」，本计划文件移除

## 5. 已知坑（防止重新踩）

- `tauri::generate_context!` 需要 `src-tauri/icons/icon.png`；`frontendDist`（ui/dist）缺失会 panic —— build.rs 已兜占位，但提交前跑 `pnpm --dir ui build`
- brew 的 `cjpeg` 是 libjpeg-turbo 的（李鬼）；mozjpeg 必须走 `(brew --prefix mozjpeg)/bin`
- libvips 子进程冷启动 ~5s → 进程池预热是 T-003 的设计要求，不是优化项
- debug 构建对 ui/dist 不校验、release 校验 —— 验证可复现性必须 `--release`
