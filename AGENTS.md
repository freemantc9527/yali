# Yali · Agent 工作守则

## 环境事实（勿重新探测）
- macOS，**无 Xcode，仅 Command Line Tools**：禁止 `swift test` / `xcodebuild` / SwiftUI 宏（CLT 不含 XCTest 与 SwiftUIMacros，已实证）
- `swiftc` 可用（仅限 helpers/ 下的小型 Swift 工具，如 Vision 封装）
- 引擎 CLI 路径以 `Brewfile` 为准，禁止假设 PATH（注意 brew 的 `cjpeg` 是 libjpeg-turbo，mozjpeg 在 keg 路径）

## 构建/测试
- 核心: `cargo test -p yali-core`（改内核必跑，目标 <60s）
- CLI: `cargo run -p yali-cli -- <args>`（安装产物统一叫 `yali`）
- UI: `npm run dev` / `npm test` / `npx playwright test`
- 整包: `cargo tauri build`（仅任务卡要求时；16GB 内存机器加 `-j4` 限流）
- 金样回归: `benchkit/bench.sh`（体积/SSIM/耗时三指标，劣化 >3% 即失败）
- workflow 安装: `workflows/install-workflows.sh`（改模板后重跑 + `killall pbs` 刷新）

## 命名权威
所有名称以 `docs/NAMING.md` 注册表为准：bundle `app.yali`、crate `yali-core/yali-cli/yali-integration`、CLI `yali`、env 前缀 `YALI_`、数据目录 `~/Library/Application Support/app.yali/`。品牌语境 Yali/鸭梨，技术语境一律小写。

## 硬性规则
- 调用 C/Vision/vips API 前，必须先 grep `Headers/` 或 `docs/api-notes/` 快照，**禁止凭记忆写 API 名**
- UI 只做任务卡所列组件，不"顺手优化"视觉；禁止无卡改样式
- 验收命令输出贴进 PR；标记 `manual-check` 的卡只能提交"待人工验收"
- 每 PR 对应一个任务卡；不混改
- `docs/` 只收录本项目自身的设计与决策文档，不引入外部来源材料

## 已知坑
- libvips 子进程冷启动 ~5s → 走进程池预热
- `.workflow` 修改后 Finder 不立即刷新 → `killall pbs`
- 非 ASCII 的 .app 包名会坑 DMG/updater → 包名固定 `Yali.app`，中文显示走 CFBundleDisplayName 本地化
