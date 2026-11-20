# 任务卡索引

一卡一任务变更 / PR，不因完成任务自动提交。验收输出留在卡片，当前状态汇总于 [STATE](../STATE.md)。历史卡的约束描述当时范围；现行 UI、构建与验证以 UI 规格、AGENTS 为准。

| 任务卡 | 内容 | 状态 |
|---|---|---|
| [T-002](T-002-profile-schema.md) | Profile Schema、Store 与出厂档 | 完成 |
| [T-003](T-003-engine-registry.md) | EngineRegistry + 子进程池 + CLI 真实现 | 完成 |
| [T-004](T-004-benchkit.md) | 合成金样 + bench.sh 三指标出表 | 完成 |
| [T-005](T-005-ui-wiring.md) | UI 接线（拖入队列 + Profile 表单） | 已实施；交互待 T-021 人工验收 |
| [T-006](T-006-m0.5-workflow.md) | Finder 右键 .workflow + 聚合通知 | 菜单与聚合通知已确认；原生通知后续见 T-026 |
| [T-007](T-007-profile-editing.md) | Profile 编辑与保存 | 完成 |
| [T-008](T-008-output-dir.md) | 输出目录策略（schema v1.1） | 完成 |
| [T-009](T-009-task-queue.md) | 任务队列并发 + 进度事件 | 完成 |
| [T-010](T-010-compare-viewer.md) | 对比查看器（滑杆 + SSIM） | 已实施，待人工验收 |
| [T-011](T-011-profile-manager.md) | Profile 管理器（另存/导入/导出） | 完成 |
| [T-012](T-012-quick-action-set.md) | 右键动作取舍（两条）+ 出厂档 jpg | 完成；两条菜单与 JPEG 隐藏规则已确认 |
| [T-013](T-013-app-bundle-open-file.md) | 真 .app + CFBundleDocumentTypes + open-file 入队 | 完成；打开方式和 Dock 已确认 |
| [T-014](T-014-settings-store.md) | 应用设置 settings.json + CLI + Tauri commands | 完成 |
| [T-015](T-015-tray.md) | 菜单栏托盘 + 关窗常驻 | 已实施，待人工验收 |
| [T-016](T-016-clipboard.md) | 剪贴板监听（复制即压） | 已实施，待人工验收 |
| [T-017](T-017-watch.md) | watch 文件夹（拖进目录即压） | 已实施，待人工验收 |
| [T-020](T-020-brand-icons.md) | 品牌应用图标集 + 透明吉祥物 + 单色菜单栏模板图 | 完成，图标已确认 |
| [T-021](T-021-responsive-workbench.md) | shadcn/ui 响应式工作台、本批参数、预设/设置/对比与原生交互 | 已实施，待人工验收（2026-11-18） |
| [T-022](T-022-preferences-storage.md) | 四个内置预设、百分比字号、精简对比与统一 `~/.yali/` | 已实施，待人工验收（2026-11-19） |
| [T-023](T-023-self-contained-app.md) | 自包含 App、内嵌 CLI / 引擎与设置页右键管理 | 已实施，待人工验收（2026-11-19） |
| [T-024](T-024-image-preview-compare.md) | 缩略图 / 适应窗口修复、原始尺寸与不同尺寸对比 | 已实施，待人工验收（2026-11-19） |
| [T-025](T-025-workbench-cache.md) | 工作台排版、同原图重复处理、有界缓存与真实文件对比 | 已实施，待人工验收（2026-11-19） |
| [T-026](T-026-native-notifications.md) | 鸭梨原生完成通知、短时发送模式与本机右键入口修复 | 已实施并安装，待人工验收（2026-11-19） |
| [T-027](T-027-versioning-workflow.md) | 完成度评估、版本定位与提交 / 推送规范 | 完成，文档变更（2026-11-19） |
| [T-028](T-028-macos-release.md) | GitHub Actions macOS 双架构构建与 Release 草稿 | 本地已实施并验证，待云端与人工验收（2026-11-19） |
| [T-029](T-029-recommended-compression.md) | 开箱即用的推荐压缩、无损优化与高级参数折叠 | 已实施并本地验证，待人工验收（2026-11-20） |

T-018 失败件持久队列与 T-019 统计页尚未实施。卡片保留设计取舍与验收证据，不删除历史卡。
