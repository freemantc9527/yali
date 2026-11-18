# 贡献指南

环境、运行与构建见 [README](README.md)；当前进度见 [STATE](docs/STATE.md)。Agent 另遵循 [AGENTS](AGENTS.md)。

- 开工先核对[任务卡](docs/tasks/README.md)，新功能先讨论范围；一个 PR 对应一张卡，附实际验证输出。
- 使用 pnpm 和入库锁文件，按改动范围运行 AGENTS 的验证命令。
- 质量相关修改须过 [benchkit](benchkit/README.md)；耗时和系统资源只观察，不为波动更新基线。
- 技术栈、工具链、分发或命名变更先写 [ADR](docs/decisions/README.md)。
- 完成后更新任务卡与 STATE；人工未验保留待验，不自动提交或推送。

流程与卡片模板见 [WORKFLOW](docs/WORKFLOW.md)。讨论对事不对人，设计取舍留在 issue / PR 或项目文档中。
