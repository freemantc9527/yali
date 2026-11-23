# ADR-002 · 技术栈：Tauri 2 + CLT-only 工具链

> 状态：**接受**（当前唯一有效技术栈决策）· 日期 2026-11-17 · 决策人：用户拍板（约束提出）+ Agent（实证）
> 取代：ADR-001

## 背景

新增硬约束：**开发机不安装 Xcode，只有 Command Line Tools**。
ADR-001 的隐含前提（`swift test`、SwiftUI 宏、xcodebuild 可用）随之崩塌，
CLT-only 下 Swift 是唯一「测试断腿」的候选：XCTest 模块缺失，SwiftUIMacros 插件不随 CLT。

## 实证（本机 CLT-only 逐项验证，2026-11-16 ~ 11-17）

| 能力 | 结果 |
|---|---|
| `swift build` / `swift run` / `swiftc` | ✅（helpers/ 小工具可用） |
| `swift test`（XCTest / swift-testing） | ❌ 全灭 |
| SwiftUI `@State` / `@Observable` 宏 | ❌ SwiftUIMacros 不随 CLT |
| AppKit 纯代码 / Vision / ImageIO / UTType / AppIntents(typecheck) | ✅ |
| ad-hoc 签名、Developer ID 签名（有证书后） | ✅ |
| Finder 快速操作（用户级 `.workflow`） | ✅ 纯文本模板，免 Xcode 免签名 |
| `cargo build -p yali-app`（Tauri 全依赖图，CLT-only） | ✅ 2026-11-17 终验通过 |
| `cargo test` | ✅ 全绿 |

## 决策

**Tauri 2（Rust 核心 + React UI），全程不依赖 Xcode。**

理由（按新约束重排）：

1. 官方依赖就是 Rust + CLT，与硬约束精确匹配，零新安装
2. Agent 测试回环完整：`cargo test` 秒级 + `pnpm test`（vitest）+ 浏览器自验证；Swift 在 CLT 下反而是唯一测试不可用的方案
3. 右键第一入口不靠 Xcode：用户级 `.workflow` → shell 节点 → CLI
4. GUI 定位本就是「参数控制台」（表单/滑条/对比器），Web UI 是强项不是妥协
5. 引擎库（oxipng / imagequant-rs / gifski）是 Rust crate，进程内链接的期权顺路可得

**放弃**：原生控件观感（用 CSS 对齐 macOS 视觉规范缓解）、SwiftUI 生态。
**兜底**：见 ADR-003（Rust 核心 + AppKit-Swift 壳，CLT 已实证可行），不回头装 Xcode。

## 后果

- 承担：WebView 老化路径需要用现代前端栈 + 定期依赖升级对冲
- 承担：包体 ~15MB（可接受，相对 Electron 是数量级改善）
- 约束：任何验收禁止出现 `swift test` / `xcodebuild`（见 [工具链边界](../TOOLCHAIN.md#1-环境与命令)）
- 回滚条件：若 M0 内核暴露 Tauri 无法覆盖的必需系统能力且 Swift helper 兜不住 → 触发 ADR-003
