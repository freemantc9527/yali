# docs/tasks · 任务卡索引

> 卡片制度：一卡一 commit；验收命令的输出摘要写进 commit message；完成即更新 `docs/STATE.md`。
> M0 四卡已全部完成（2026-11-17），出口条件见 `docs/STATE.md`。

| 卡 | 状态 | 内容 | 验收 |
|---|---|---|---|
| [T-002-profile-schema.md](T-002-profile-schema.md) | ✅ 完成（2026-11-17） | Profile v1 六段 Schema + Store + 五档出厂 fixtures | `cargo test -p yali-core profile` 15+2 绿；CLI `show default` 与 fixture 逐字节一致 |
| [T-003-engine-registry.md](T-003-engine-registry.md) | ✅ 完成（2026-11-17） | EngineRegistry + 子进程池 + CLI 真实现 | doctor 11 引擎探测 ✓；compress 冒烟 p50≈95ms；负优化拦截/undo 还原均有测试；32+2 测试全绿 |
| [T-004-benchkit.md](T-004-benchkit.md) | ✅ 完成（2026-11-17） | 合成金样 + bench.sh 三指标出表 | generate.sh 确定性可复跑；bench 双档通过；劣化>3% 实测退出码 1；baseline 入库 |
| [T-005-ui-wiring.md](T-005-ui-wiring.md) | ✅ 完成（2026-11-17） | UI 接线（拖入队列 + Profile 表单） | 19 个前端测试绿；command 层真压缩冒烟；浏览器端到端拖入→结果行；真机拖拽待人工复核 |
| [T-006-m0.5-workflow.md](T-006-m0.5-workflow.md) | ✅ 完成（2026-11-17，manual 待复核） | Finder 右键 .workflow + 聚合通知 | automator 端到端产出 + pbs 注册可见；CLI 聚合文案三态单测；右键菜单出现属人工验收 |
| [T-007-profile-editing.md](T-007-profile-editing.md) | ✅ 完成（2026-11-17） | Profile 编辑与保存 | `profiles save` 往返 + 非法值拒绝；command 冒烟含 save；ui 29 测试绿；浏览器实测保存/错误提示 |
| [T-008-output-dir.md](T-008-output-dir.md) | ✅ 完成（2026-11-17） | 输出目录策略（schema v1.1） | 四策略 + `{size}` 实测；undo 在 custom 目录还原；ui 31 测试绿 |
| [T-009-task-queue.md](T-009-task-queue.md) | ✅ 完成（2026-11-17） | 任务队列并发 + 进度事件 | 事件 started/done/failed + reducer 乱序测试；浏览器 3 文件逐行更新实测 |
| [T-011-profile-manager.md](T-011-profile-manager.md) | ✅ 完成（2026-11-17） | Profile 管理器（另存/导入/导出） | 纯函数 12 测；浏览器实测另存与非法 JSON 报错 |
| [T-010-compare-viewer.md](T-010-compare-viewer.md) | ✅ 完成（2026-11-17，manual 待复核） | 对比查看器（滑杆 + SSIM） | metrics 测试（同图=1、低质副本<0.95）；ui 48 测试；应用启动冒烟；真机滑杆手感人工验收 |
| [T-012-quick-action-set.md](T-012-quick-action-set.md) | ✅ 完成（2026-11-17，manual 待复核） | 右键三件套 + 出厂档 jpg | automator 三动作各自产出正确；RGBA→JPG 白底正确；GIF→JPG 不再缩小；bench 三档绿 |

T-002 落地物：`schema/profile.schema.json`（文档事实源）、`fixtures/`（五档出厂）、`crates/yali-core/src/{profile,store}.rs`。
