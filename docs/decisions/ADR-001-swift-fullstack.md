# ADR-001 · 技术栈：Swift 全家桶（SwiftUI + SwiftPM）

> 状态：**已废弃**（同日被 ADR-002 取代，保留作决策审计）· 日期 2026-11-16
> 废弃原因：隐含前提「开发机有 Xcode」不成立。真实约束是**不安装 Xcode**，见 ADR-002。

## 背景

产品价值的大头在 macOS 系统集成（右键快速操作、剪贴板、watch、Vision 抠图、ImageIO 硬编），
且本项目由 AI Agent 执行开发，选型权重需要显式加入「Agent 生产力」这一维度。

权重：系统集成深度 40% · Agent 开发效率 30% · 长期维护成本 15% · 性能/包体 10% · 跨平台 5%。

## 候选与打分（加权总分）

| 方案 | 集成 | Agent | 维护 | 性能 | 跨平台 | 总分 |
|---|---|---|---|---|---|---|
| A · Swift 全家桶（SwiftUI + SwiftPM 核心） | 10 | 7 | 10 | 9 | 2 | **8.7** |
| B · Tauri 2（Rust 核心 + Web UI） | 4 | 9 | 6 | 7 | 9 | 6.4 |
| C · Rust 核心 + Swift 壳 | 10 | 6 | 9 | 9 | 4 | 8.3 |
| D · Electron（对照组） | 5 | 8 | 3 | 4 | 10 | 5.9 |
| E · Flutter / KMP 桌面 | 否决（系统集成比 Tauri 更薄，语料更少） | | | | | |

## 决策

选 A（Swift 全家桶），并附带三条工程约束：

1. 核心逻辑 100% 放 SwiftPM 包，`swift test` 作为 Agent 主回环，App target 只做渲染壳
2. 用 XcodeGen 管理工程，仓库只提交 `project.yml`（pbxproj 不入库，避免 Agent 改工程文件翻车）
3. 「引擎进程内链接」推迟为可选优化：Phase A 子进程架构实测单图 50 到 500ms，进程池预热后差异 <20ms。

## 后果

- 放弃：Windows/Linux 可能性（Pipeline/Profile 层仍保持平台无关，留期权）
- 承担：Agent 对 SwiftUI 细节偶发幻觉 → 用组件清单 + 快照测试约束
- 承担：Xcode 版本绑定

## 复盘（ADR-002 时写下）

1. 把「Swift 全家桶」的测试、宏和打包能力默认当作 CLT 自带，实际是 Xcode 独占
2. 把「必须装 Xcode」当成环境修复项，而不是选型约束
3. 高估了 Tauri 的集成劣势：`.workflow` + CLT 编译的 Swift helper + 系统 CLI 足以覆盖护城河功能
