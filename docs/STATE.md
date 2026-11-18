# STATE · 当前项目状态

> 更新：2026-11-18。每卡完成时更新，历史证据留在[任务卡](tasks/README.md)。开始先核对工作区与当前任务，不自动 pull、建分支或提交。

## 1. 进度

| 阶段 | 当前状态 |
|---|---|
| M-1 / M0 | CLT-only 构建、内核、CLI、金样与 undo 已通过（2026-11-17） |
| M0.5 | 两条 Finder 快速操作、多选聚合通知已获用户确认（2026-11-17）；通知点击行为待验 |
| M1 | 原 GUI 与 open-file 已实现；T-021 响应式重设计已实施，原生交互待人工验收 |
| M2 | 设置、托盘、剪贴板、watch 已实现；托盘菜单与监听场景仍待人工验收；T-018 失败件队列、T-019 统计页未实施 |
| 品牌 | T-020 应用与菜单栏图标已获用户确认（2026-11-18） |
| M3 / M4 | 智能分类、参数标定、Vision、正式分发与 updater 未实施 |

## 2. 本轮交付：T-021

规格见 [UI-REDESIGN](UI-REDESIGN.md)，选型见 [ADR-005](decisions/ADR-005-shadcn-responsive-ui.md)，命令输出、金样、真实投递和资源观察见 [T-021](tasks/T-021-responsive-workbench.md)。

- 已实现 shadcn/ui + Tailwind 4 工作台、本批草稿、预设、设置、对比、batch/job 身份、逐项结果和受限预览缓存，未改 core/CLI 算法与 Schema。
- 本轮审查修复重试入队丢任务、TIFF 自动输出重名漏检；隔离副本前端 61 项、应用 20 项测试通过，UI 构建及整包 / 10 类 UTI 校验通过。
- 浏览器覆盖 1280/980/800/640/390 与断点两侧；历史真实应用两次投递共 10 图，10/10 产物。浏览器演示不替代原生人工验收。
- README 面向使用者；AGENTS 面向 Agent，含 CodeGraph 优先规则；新增 `scripts/dev.sh` 与 `scripts/build.sh`。
- 金样体积 / SSIM 无劣化。用户已确认耗时、内存、CPU 仅观察，脚本不再以耗时判失败，不改质量基线。
- 用户后续授权提交全部变更：提交到当前 dev 分支，不创建分支、不 push；人工待验状态不变。

## 3. 人工验收与下一步

- [x] Finder 有且仅有两条鸭梨动作，JPEG 不显示「转 JPG」；多选一条聚合通知（用户确认，2026-11-17）。
- [x] Finder「打开方式」出现 Yali，Dock 拖图标入队（用户确认，2026-11-17）。
- [x] 应用与菜单栏图标显示、尺寸和辨识度（用户确认，2026-11-18）。
- [ ] T-021 真 Tauri 选择 / 取消 / 拖入、草稿真实处理、预设写盘影响、Finder 定位、设置生效、WKWebView 对比 / 窗口 / 键盘手感，详见卡片。
- [ ] T-015 托盘菜单展开 / 唤回，T-016 剪贴板，T-017 watch 的实际交互与文件写完后处理。
- [ ] 通知点击焦点行为；产物在微信 / 浏览器 / Preview 的兼容抽验。

先完成相关人工验收，再推进 T-018 → T-019；M2 出口检查 20 图右键聚合通知，耗时只观察。`rusqlite` 尚未引入；`helpers/`、`crates/yali-integration`、M3 `calibration.json` 尚未创建，需任务卡后实施。

## 4. 按需阅读

- 产品与命名：[DESIGN](DESIGN.md)、[NAMING](NAMING.md)；决策理由：[ADR](decisions/README.md)。
- 执行与验证：[AGENTS](../AGENTS.md)、[WORKFLOW](WORKFLOW.md)；本地运行：[README](../README.md)。
- 构建与 Tauri 坑：[TOOLCHAIN](TOOLCHAIN.md)；图像与质量坑：[benchkit](../benchkit/README.md)；Finder 安装坑：[workflows](../workflows/README.md)。
- 平台方向：[CROSSPLATFORM](CROSSPLATFORM.md)，未排期。
