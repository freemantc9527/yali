# Contributing

欢迎！本项目由人类 + AI Agent 协作开发，贡献流程沿用任务卡制。

## 开始

```bash
brew bundle --file Brewfile            # 引擎依赖（pnpm/node 由开发机自备，见 Brewfile 注释）
cargo test                             # 内核测试（<60s）
pnpm --dir ui install                  # 前端依赖
pnpm --dir ui test                     # 前端单测
```

## 规则
1. 先读 `AGENTS.md`（人类同样适用——它就是工程约定）与 `docs/STATE.md`（当前进度）
2. 新功能先看任务卡是否已存在（`docs/tasks/` 计划中，T-004 起落盘）；没有就开 issue 讨论后再动代码
3. 一个 PR 对应一个任务卡；PR 描述必须附验收命令的输出
4. 涉及压缩质量的变化必须过金样回归（`benchkit/bench.sh`；劣化 >3% 拒绝合入）
5. 技术栈 / 工具链 / 命名变更先写 ADR（`docs/decisions/`）

## 好上手的第一批任务
- 引擎适配（`EngineRegistry` 注册新 CLI，T-003 起）
- 标定扫描（参数网格 → `calibration.json`，M3）
- 金样素材扩充（`benchkit/generate.sh`，确定性可复跑）

## 前端约定
包管理器是 **pnpm**（ADR-004）：`pnpm --dir ui …`，不要用 `npm` / `npx`（需要执行本地二进制时用 `pnpm exec`）。

## 行为准则
讨论对事不对人；issue/PR 是设计讨论的唯一场所（没有秘密聊天式设计）。
