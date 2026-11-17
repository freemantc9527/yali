# docs/tasks · 任务卡索引

> 卡片制度：一卡一 commit；验收命令的输出摘要写进 commit message；完成即更新 `docs/STATE.md`。
> 执行顺序与依赖见 `docs/plans/PLAN-M0.md`。

| 卡 | 状态 | 内容 | 验收 |
|---|---|---|---|
| [T-002-profile-schema.md](T-002-profile-schema.md) | ✅ 完成（2026-11-17） | Profile v1 六段 Schema + Store + 五档出厂 fixtures | `cargo test -p yali-core profile` 15+2 绿；CLI `show default` 与 fixture 逐字节一致 |
| [T-003-engine-registry.md](T-003-engine-registry.md) | ⬜ 待开工 | EngineRegistry + 子进程池 + CLI 真实现 | `yali compress` 单图冒烟；负优化拦截；undo 可还原 |
| [T-004-benchkit.md](T-004-benchkit.md) | ⬜ 待开工 | 合成金样 + bench.sh 三指标出表 | 金样生成 + 基线入库 |
| [T-005-ui-wiring.md](T-005-ui-wiring.md) | 🔶 脚手架已建 | UI 接线（拖入队列 + Profile 表单） | 拖入 jpg → 产出 `-{profile}` 文件（manual-check） |

T-002 落地物：`schema/profile.schema.json`（文档事实源）、`fixtures/`（五档出厂）、`crates/yali-core/src/{profile,store}.rs`。
