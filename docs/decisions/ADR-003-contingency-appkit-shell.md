# ADR-003 · 兜底方案：Rust 核心 + AppKit-Swift 壳

> 状态：**备而未用（contingency）** · 日期 2026-11-17
> 说明：这不是待执行计划，是 ADR-002 的回滚路径。未触发，不投入任何工期。

## 背景

ADR-002 押注 Tauri 2 覆盖系统集成能力。若 M0/M0.5 出现 Tauri 无法实现、
且 CLI 子进程 / 小型 Swift helper 也兜不住的**必需**能力，需要一条不装 Xcode 的退路。

## 方案

- 内核不变：`yali-core` / `yali-cli`（Rust）原样保留，验收命令不变
- 壳替换：Tauri 壳 → AppKit 纯代码 Swift 壳（CLT 的 `swiftc` 编译，`codesign -s -` ad-hoc 签名）
- UI 损失：参数控制台需用 AppKit 重写（这是本方案的真实成本，也是不轻易触发的原因）

## 触发条件（任一满足才升级讨论）

1. 某个护城河能力（右键/剪贴板/watch/Vision/ImageIO）在 Tauri + helper 组合下不可行
2. Tauri 壳在目标 macOS 版本上出现不可绕过的 WebView 缺陷

## 后果

- 已实证可用：AppKit 纯代码、Vision、ImageIO、UTType 在 CLT-only 下均可编译运行
- 成本：GUI 层重写；核心与 CLI 零改动（这是分层设计的目的）
- 若触发，需另立 ADR 记录实际理由，不直接在本文件上改写
