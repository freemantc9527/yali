# ADR · 架构决策记录

> 规则：任何影响**技术栈、分发形态、命名、构建工具链**的变更，先在此留一份 ADR 再动代码。
> 命名以 [NAMING](../NAMING.md) 为准。
> 状态取值：`接受` / `已废弃（superseded by ADR-xxx）` / `备而未用（contingency）` / `已否决`。

## 索引

| 编号 | 标题 | 状态 |
|---|---|---|
| [ADR-001](ADR-001-swift-fullstack.md) | 技术栈：Swift 全家桶 | 已废弃（被 ADR-002 取代） |
| [ADR-002](ADR-002-tauri-clt-only.md) | 技术栈：Tauri 2 + CLT-only 工具链 | 接受 |
| [ADR-003](ADR-003-contingency-appkit-shell.md) | 兜底：Rust 核心 + AppKit-Swift 壳 | 备而未用（未触发） |
| [ADR-004](ADR-004-pnpm.md) | 前端包管理器：pnpm | 接受 |
| [ADR-005](ADR-005-shadcn-responsive-ui.md) | GUI：shadcn/ui + Tailwind CSS 4 响应式工作台 | 接受，部分原生交互待人工验收 |
| [ADR-006](ADR-006-self-contained-app.md) | 自包含个人学习版 App、内嵌 CLI 与右键操作管理 | 接受 |
| [ADR-007](ADR-007-github-macos-release.md) | GitHub Actions macOS 双架构构建与 Release 草稿 | 接受，云端验证待完成 |

## 新 ADR 模板

```markdown
# ADR-00X · 标题

> 状态：接受 / 已废弃 / 备而未用 · 日期 YYYY-MM-DD · 决策人（人 / Agent）

## 背景
（触发的约束或新事实，只写可验证的部分）

## 决策
（一句话结论，然后列出被放弃的选项）

## 后果
- 承担：
- 放弃：
- 回滚条件：（什么情况下重新评估）
```
