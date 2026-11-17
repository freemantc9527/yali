# Contributing

欢迎！本项目由人类 + AI Agent 协作开发，贡献流程沿用任务卡制。

## 开始

```bash
brew bundle --file Brewfile     # 引擎依赖
cargo test                       # 内核测试（<60s）
npm --prefix ui install && npm --prefix ui test   # UI（M1 起）
```

## 规则
1. 先读 `AGENTS.md`（人类同样适用——它就是工程约定）与 `docs/STATE.md`（当前进度）
2. 新功能先看 `docs/tasks/` 是否已有任务卡；没有就开 issue 讨论后再动代码
3. 一个 PR 对应一个任务卡；PR 描述必须附验收命令的输出
4. 涉及压缩质量的变化必须过 `benchkit/bench.sh` 金样回归（劣化 >3% 拒绝合入）

## 好上手的第一批任务
- 引擎适配（`EngineRegistry` 注册新 CLI）
- 标定扫描（参数网格 → `calibration.json`）
- 金样素材扩充（合成图脚本，见 benchkit/）

## 行为准则
讨论对事不对人；issue/PR 是设计讨论的唯一场所（没有秘密聊天式设计）。
