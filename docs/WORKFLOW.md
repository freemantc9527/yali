# WORKFLOW · 任务卡制与验收流程

> 本项目由人类 + AI Agent 协作开发。本文定义：怎么拆任务、怎么验收、哪些必须人来确认。

## 1. 核心制度

1. **内核优先**：所有逻辑必须可在终端被验收（headless）；GUI 只做渲染
2. **契约先行**：跨模块依赖一律先落 `schema/`（profile.schema.json + fixtures）+ 接口文档，再写实现
3. **一卡一 PR**：验收 = 可执行命令的输出，不接受描述性"已完成"
4. **状态接力**：`docs/STATE.md` 是跨 session 冷启动的唯一入口，每卡完成即更新
5. **金样守门**：压缩质量变更必须过 `benchkit/bench.sh`（体积/SSIM/耗时，劣化>3% 拒绝合入；脚本随 T-004 建立）

## 2. 任务卡模板（`docs/tasks/T-xxx.md`，该目录随 T-004 落地）

```markdown
# T-012 · Profile Schema 与 Store
goal: Profile v1 JSON Schema + 加载/保存/迁移
context: docs/DESIGN.md §2；schema/fixtures/
constraints:
  - 纯 Rust crate（yali-core），零 UI 依赖
  - 不新增第三方依赖（白名单见 AGENTS.md）
acceptance:
  - `cargo test -p yali-core profile` 绿
  - `yali profiles show default` 输出与 fixtures/default.json 逐字段一致
  - 非法 JSON 报错含行号
estimated: 2h
manual-check: false
```

## 3. 人工验收点（Agent 无法自验证，卡内标 `manual-check: true`）

| 项 | 时机 |
|---|---|
| Finder 快速操作真实出现、多选生效、通知文案/点击 | M0.5 末 |
| 拖 Dock / 打开方式进队列 | M1 末 |
| GUI 视觉对照 DESIGN.md §5 规格 | M1/M3 末 |
| 产物在微信/浏览器/Preview 显示 | 每里程碑抽验 |
| 剪贴板监听 / watch 实时性 | M2 末 |

## 4. 回环预算

- `cargo test` <60s（改核心必跑）；`pnpm --dir ui test` 秒级；Vite dev 随便跑
- `cargo tauri build` 批处理化：一天攒几个 UI 卡再统一整包构建（16GB 机器 `-j4`）
- UI 任务禁止无卡样式迭代（防"美化"黑洞）

## 5. 里程碑序列（详见 DESIGN.md §8）

```
M-1 ✅ → M0 契约+内核+CLI → M0.5 右键闭环 → M1 GUI → M2 入口全量 → M3 智能 → M4 发布
```

## 6. 坑账本制度

STATE.md 的「坑账本」段随踩随记（如：generate_context! 编译期要图标、brew cjpeg 是李鬼、killall pbs 刷新 workflow）。同类问题二次出现必须写进 AGENTS.md 硬性规则。
