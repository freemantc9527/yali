# TOOLCHAIN · 构建工具链约束与实证边界

> 本项目的**硬约束：全程不依赖 Xcode**（开发机仅 Command Line Tools）。这不是偏好，是选型前提——所有工程决策以此为锚。本文记录实证边界，防止后续 session 重新探测或误引 Xcode 依赖。

## 1. 定稿技术栈

**Tauri 2（Rust 核心 + React UI）+ CLT-only 工具链** —— 2026-11-17 终验通过：`cargo build -p yali-app`（含 tauri/objc2/webkit 全依赖图）编译成功，`cargo test` 全绿。决策全文见 `docs/decisions/ADR-002-tauri-clt-only.md`（兜底路径 ADR-003 备而未用）。
前端包管理器 **pnpm**（ADR-004），Tauri 钩子命令统一 `pnpm --dir ui …`。

选型排除逻辑（摘要）：
- 依赖 SwiftUI/SwiftPM 全家桶 ❌——CLT 无 XCTest/swift-testing（`swift test` 不可用）、无 SwiftUIMacros（@State/@Observable 宏不可编译）
- Electron ❌——包体/内存/老化路径与"原生轻量"定位相反
- Tauri ✅——官方依赖即 Rust+CLT；测试回环（cargo test + playwright）完整；系统能力经 CLI/小 helper 覆盖

## 2. CLT-only 能力边界（逐项实证）

| 能力 | 状态 | 备注 |
|---|---|---|
| `swift build` / `swift run` / `swiftc` | ✅ | helpers/ 下小型 Swift 工具可用（Vision 封装已实证编译+运行） |
| `swift test`（XCTest / swift-testing） | ❌ | CLT 缺框架——**任何验收不得出现 swift test** |
| SwiftUI @State/@Observable 宏 | ❌ | SwiftUIMacros 插件不随 CLT；无宏 SwiftUI 可编译但不实用 |
| AppKit 纯代码 | ✅ | 托盘拖入等补充能力可用 20 行 helper 实现 |
| Vision / ImageIO / UTType / AppIntents(typecheck) | ✅ | |
| ad-hoc 签名 `codesign -s -` | ✅ | 本地开发/workflow 安装不依赖证书 |
| Developer ID + notarytool | ✅（有证书后） | CLT 自带工具；**证书已决定延后**，不阻塞任何里程碑 |
| Finder 快速操作（用户级 .workflow） | ✅ | 纯文本模板生成，免 Xcode 免签名 |
| xcodebuild / 资产目录 actool / Mac App Store | ❌ | 不在路线图上（分发走 Developer ID 直发 + brew） |

## 3. 引擎依赖（Brewfile 钉版，本机已装）

mozjpeg 4.1.5（⚠️ keg-only：brew 的 `cjpeg` 是 libjpeg-turbo 3.2.1 的，mozjpeg 必须用 `(brew --prefix mozjpeg)/bin` 路径）· webp 1.6.0 · jpeg-xl 0.13.0 · libavif 1.4.2 + aom 3.15.3 + svt-av1 4.3.0 · pngquant 3.0.3 · oxipng 10.2.1 · gifsicle 1.98 · gifski 1.34 · vips 8.18.7 · imagemagick 7.1.2（仅度量）

## 4. 工程约定（由工具链约束推导）

1. 核心逻辑 100% Rust（SwiftPM 不可作为主栈，见 §1）；Swift 仅允许出现在 `helpers/` 的独立小工具
2. 验收命令白名单：`cargo test` / `cargo run -p yali-cli` / `pnpm --dir ui test` / `pnpm exec playwright test`（M1 起）/ `benchkit/bench.sh`（T-004 起）
3. 16GB 内存机：`cargo build -j4`；GUI 整包构建批处理化（Vite dev 秒级回环不受限）
4. CI 保持与本地一致的 CLT-only 命令集（GitHub macOS runner 虽有 Xcode，不使用，防"CI 能过本地不能过"）
5. `tauri::generate_context!` 编译期硬依赖 `src-tauri/icons/icon.png`（bundle.active=false 也要）——新克隆先有图标
6. `tauri::generate_context!` 在**非 dev** 构建时硬校验 `frontendDist`（`../ui/dist`）存在，缺失直接 panic（`tauri-codegen` 的 codegen 阶段，不是运行时）。`ui/dist` 是构建产物不入库，因此 `src-tauri/build.rs` 在 codegen 之前兜一个占位 `index.html`；正式提交前仍应跑 `pnpm --dir ui build` 产出真前端
7. debug 构建对 `frontendDist` 不校验（`dev_url` 有值即短路）——**不要**因此认为「新克隆能 debug 编译 = 仓库可复现 release」
