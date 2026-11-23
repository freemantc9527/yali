# 项目文档

这里保存可以长期引用的项目资料。临时任务、实现草稿、实验日志和验收过程放在仓库根目录的 `.tasks/`，该目录被 Git 忽略，不会出现在公开文档导航中。

| 文档 | 作用 |
| --- | --- |
| [DEVELOPMENT](DEVELOPMENT.md) | 环境准备、运行、测试和调试入口 |
| [ARCHITECTURE](ARCHITECTURE.md) | Rust 内核、CLI、Tauri、UI 和系统入口 |
| [DESIGN](DESIGN.md) | 产品行为、Profile、数据和安全边界 |
| [UI-REDESIGN](UI-REDESIGN.md) | 当前工作台交互与响应式规则 |
| [TOOLCHAIN](TOOLCHAIN.md) | 构建工具、运行时和本机约束 |
| [CROSSPLATFORM](CROSSPLATFORM.md) | macOS 主线与 Windows 实验范围 |
| [NAMING](NAMING.md) | 品牌、包名、CLI 和环境变量命名 |
| [WORKFLOW](WORKFLOW.md) | 任务卡、验收、归档和变更记录流程 |
| [VERSIONING](VERSIONING.md) | 版本号、分支和源码追溯 |
| [RELEASING](RELEASING.md) | GitHub Actions、产物和发布门槛 |
| [CHANGELOG](CHANGELOG.md) | 面向用户的版本变化 |
| [STATE](STATE.md) | 当前状态、已知限制和人工验收边界 |
| [BENCHMARKS](BENCHMARKS.md) | 图片质量基线和 benchkit 说明 |
| [decisions](decisions/README.md) | 已接受的架构决策 |

文档只描述已经存在或明确计划中的行为。若文档与代码冲突，以可验证的实现和最近一次任务记录为准，并在修复实现的同时更新文档。
