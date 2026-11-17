# PLAN · M1 GUI（执行计划，M1 收口后删除）

> 前置：M0.5 右键闭环自动部分完成（T-006）；GUI 目标见 docs/DESIGN.md §5。
> 制度不变：一卡一 commit、验收命令化、UI 禁止无卡样式迭代。

## 卡片

| 卡 | 内容 | 验收 |
|---|---|---|
| T-007 ✅ | Profile 编辑与保存（GUI 表单可写 + 原子落盘 + 校验错误展示） | `yali profiles save` 往返；`pnpm --dir ui test` 编辑逻辑单测；非法值在 UI 报错不落盘 |
| T-008 ✅ | 输出目录策略落地（sibling / downloads / custom+dirPath，schema v1.1） | schema/fixtures 同步；管道单测三策略；CLI 实测 custom 目录 |
| T-009 ✅ | 任务队列与并发（min(cores-1,4)）+ 进度事件 | 10 图批处理不阻塞 UI；事件流单测 |
| T-010 | 对比查看器（before/after 滑杆 + SSIM 标注） | ✅ 自动部分完成；真机滑杆手感人工验收 |
| T-011 ✅ | Profile 管理器（另存 / 导入导出 JSON） | 导入非法 JSON 报错含行号；导出可被 CLI 消费 |

## 依赖
T-007 → T-008 → T-009 → T-010/T-011

## 出口条件（DESIGN §8 M1）
- §5 覆盖（六卡 Inspector / 对比查看器 / Profile 管理器）
- 10 图批处理内存峰值 < 300MB（Instruments 抽验，人工）
- `cargo tauri dev` 真机全流程可用（人工）

## 已知坑
- 图标必须是 RGBA8（gray+alpha 会让 Tauri 启动 panic）
- Services 上下文 PATH 极简（见坑账本）
- 16GB 机器：UI 卡攒批再整包构建，日常用 `cargo tauri dev`

## 状态（2026-11-17）：五卡全部落地

- 自动验收全绿：cargo 44 + ui 48；bench default/web 双档通过（含 perf 回归修复：尺寸守卫收窄 + 并发探测）
- **人工验收待复核**：真机 `cargo tauri dev` 全流程（拖入→结果→对比滑杆）、10 图批处理内存峰值 <300MB
