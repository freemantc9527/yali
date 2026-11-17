# docs/tasks · 任务卡索引

> 卡片制度：一卡一 commit；验收命令的输出摘要写进 commit message；完成即更新 `docs/STATE.md`。
> M0 四卡已全部完成（2026-11-17），出口条件见 `docs/STATE.md`。

| 卡 | 状态 | 内容 | 验收 |
|---|---|---|---|
| [T-002-profile-schema.md](T-002-profile-schema.md) | ✅ 完成（2026-11-17） | Profile v1 六段 Schema + Store + 五档出厂 fixtures | `cargo test -p yali-core profile` 15+2 绿；CLI `show default` 与 fixture 逐字节一致 |
| [T-003-engine-registry.md](T-003-engine-registry.md) | ✅ 完成（2026-11-17） | EngineRegistry + 子进程池 + CLI 真实现 | doctor 11 引擎探测 ✓；compress 冒烟 p50≈95ms；负优化拦截/undo 还原均有测试；32+2 测试全绿 |
| [T-004-benchkit.md](T-004-benchkit.md) | ✅ 完成（2026-11-17） | 合成金样 + bench.sh 三指标出表 | generate.sh 确定性可复跑；bench 双档通过；劣化>3% 实测退出码 1；baseline 入库 |
| [T-005-ui-wiring.md](T-005-ui-wiring.md) | ✅ 完成（2026-11-17） | UI 接线（拖入队列 + Profile 表单） | 19 个前端测试绿；command 层真压缩冒烟；浏览器端到端拖入→结果行；真机拖拽待人工复核 |

T-002 落地物：`schema/profile.schema.json`（文档事实源）、`fixtures/`（五档出厂）、`crates/yali-core/src/{profile,store}.rs`。
