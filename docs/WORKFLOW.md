# WORKFLOW · 任务与验收

## 核心制度

- 内核逻辑可在终端验证；跨模块数据变更先确认 Schema / fixtures 与接口合同。
- 一卡一 PR，验收依据是实际命令输出；不因卡片完成自动创建分支、提交或推送。
- 版本定位、提交格式、分支与推送规则见 [VERSIONING](VERSIONING.md)；制定规范不授权执行 Git 交付或发布。
- 卡片保留范围、约束、验收证据与人工待验；STATE 只记录当前事实。
- 改动范围对应的必跑命令见 [AGENTS](../AGENTS.md)。验证必须查看实际退出状态，不能只用过滤后的计数宣称通过。
- 压缩质量改动先重建所测 CLI 再跑 [benchkit](../benchkit/README.md)。体积 / SSIM 是硬门槛，耗时、内存、CPU 只观察。
- UI 只按任务卡调整；技术栈 / 命名 / 分发 / 工具链变化先写 ADR。

## 任务卡模板

```markdown
# T-xxx · 标题

goal: 可观察的结果
context: 相关规格、决策与上游任务
constraints:
  - 范围与不可破坏的约束
acceptance:
  - 实际命令及预期结果
  - 人工验收场景（如适用）
manual-check: false

## 验收记录
日期、实际输出摘要、已确认项与剩余项。
```

## 人工验收

Finder 菜单 / 通知点击、Dock 投递、GUI 视觉与操作手感、剪贴板 / watch 交互、产物兼容性由用户确认。自动化与浏览器演示只证明各自路径，不能替代人工验收。已确认与待验状态见 [STATE](STATE.md) 和各任务卡。

稳定的构建坑写 [TOOLCHAIN](TOOLCHAIN.md)，图像度量坑写 [benchkit](../benchkit/README.md)，系统安装坑写 [workflows](../workflows/README.md)；只有影响日常执行的短规则放 AGENTS，避免重复堆叠。
