# Yali · Agent 工作守则

## 环境事实（勿重新探测）
- macOS，**无 Xcode，仅 Command Line Tools**：禁止 `swift test` / `xcodebuild` / SwiftUI 宏（CLT 不含 XCTest 与 SwiftUIMacros，已实证，见 `docs/TOOLCHAIN.md` §2）
- `swiftc` 可用（仅限未来 `helpers/` 下的小型 Swift 工具，如 Vision 封装）
- 引擎 CLI 路径以 `Brewfile` 为准，禁止假设 PATH（注意 brew 的 `cjpeg` 是 libjpeg-turbo，mozjpeg 在 keg 路径）
- 前端包管理器是 **pnpm**（ADR-004），文档与命令里不出现 `npm` / `npx`

## 构建/测试
| 目标 | 命令 | 现状 |
|---|---|---|
| 内核 | `cargo test -p yali-core`（改内核必跑，目标 <60s） | 可用 |
| CLI | `cargo run -p yali-cli -- <args>`（安装产物统一叫 `yali`） | 可用 |
| UI | `pnpm --dir ui test` / `pnpm --dir ui dev` | 可用（T-005 接线） |
| UI 构建 | `pnpm --dir ui build` → `ui/dist` | 可用 |
| 整包 | `cargo tauri build`（仅任务卡要求时；16GB 机器加 `-j4`） | 前端产物齐备后可用 |
| 金样回归 | `benchkit/bench.sh`（体积/SSIM/耗时三指标，劣化 >3% 即失败） | 可用（T-004） |
| workflow 安装 | `workflows/install-workflows.sh`（改模板后重跑 + `killall pbs`） | 可用（T-006） |

## 命名权威
所有名称以 `docs/NAMING.md` 注册表为准：bundle `app.yali`、crate `yali-core/yali-cli`、CLI `yali`、env 前缀 `YALI_`、数据目录 `~/Library/Application Support/app.yali/`。品牌语境 Yali/鸭梨，技术语境一律小写。
`crates/yali-integration`（平台差异收口）与 `crates/yali-vision`（M3）**尚未创建**，是预注册名，不要在未建卡的 PR 里提前落地。

## 目录现状（勿假设不存在的东西）
已存在：`crates/yali-core`、`crates/yali-cli`、`src-tauri/`、`ui/`、`docs/`（含 `docs/decisions/`、`docs/tasks/`）、`schema/`、`fixtures/`、`benchkit/`、`workflows/`。
**尚未存在**（按任务卡创建，见 `docs/STATE.md`）：`helpers/`、`crates/yali-integration`。
调用前先 `ls`，不要凭记忆引用不存在的路径。

## 硬性规则
- 调用 C/Vision/vips API 前，必须先 grep 系统头文件（`/Library/Developer/CommandLineTools/SDKs/*/usr/include/`）或本机实测，**禁止凭记忆写 API 名**（`docs/api-notes/` 尚未建立，等 M3 有了再引用）
- UI 只做任务卡所列组件，不"顺手优化"视觉；禁止无卡改样式
- 验收命令输出贴进 PR；标记 `manual-check` 的卡只能提交"待人工验收"
- 每 PR 对应一个任务卡；不混改
- `docs/` 只收录本项目自身的设计与决策文档，不引入外部来源材料

## 已知坑
- libvips 子进程冷启动 ~5s → 走进程池预热
- `.workflow` 修改后 Finder 不立即刷新 → `killall pbs`
- 非 ASCII 的 .app 包名会坑 DMG/updater → 包名固定 `Yali.app`，中文显示走 CFBundleDisplayName 本地化
- `tauri::generate_context!` 编译期需要 `src-tauri/icons/icon.png`（bundle.active=false 也要）
- `frontendDist`（`ui/dist`）在**非 dev** 构建时是硬校验：目录不存在直接 panic。`src-tauri/build.rs` 已兜占位页，但提交前务必跑一次 `pnpm --dir ui build` 让产物是真前端
