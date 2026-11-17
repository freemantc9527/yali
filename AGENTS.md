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
| 整包 | `env -u CI cargo tauri build --bundles app` + `scripts/bundle-check.sh`（T-013 实测） | 可用（tauri-cli 2.12.5） |
| 金样回归 | `benchkit/bench.sh`（体积/SSIM/耗时三指标，劣化 >3% 即失败） | 可用（T-004） |
| workflow 安装 | `workflows/install-workflows.sh`（改模板后重跑 + `killall pbs`） | 可用（T-006） |

## 命名权威
所有名称以 `docs/NAMING.md` 注册表为准：bundle `app.yali`、crate `yali-core/yali-cli`、CLI `yali`、env 前缀 `YALI_`、数据目录 `~/Library/Application Support/app.yali/`。品牌语境 Yali/鸭梨，技术语境一律小写。
`crates/yali-integration`（平台差异收口）与 `crates/yali-vision`（M3）**尚未创建**，是预注册名，不要在未建卡的 PR 里提前落地。

## 目录现状（勿假设不存在的东西）
已存在：`crates/yali-core`、`crates/yali-cli`、`src-tauri/`（含 `capabilities/`、`Info.plist`）、`ui/`、`docs/`（含 `docs/decisions/`、`docs/tasks/`）、`schema/`、`fixtures/`、`benchkit/`、`workflows/`、`scripts/`。
**尚未存在**（按任务卡创建，见 `docs/STATE.md`）：`helpers/`、`crates/yali-integration`。
调用前先 `ls`，不要凭记忆引用不存在的路径。

## 硬性规则
- **默认只本地 commit，禁止 push**：`git push` 必须等用户明确指令（2026-11-17 用户定，任何 PR/卡完成都不例外）
- 调用 C/Vision/vips API 前，必须先 grep 系统头文件（`/Library/Developer/CommandLineTools/SDKs/*/usr/include/`）或本机实测，**禁止凭记忆写 API 名**（`docs/api-notes/` 尚未建立，等 M3 有了再引用）
- UI 只做任务卡所列组件，不"顺手优化"视觉；禁止无卡改样式
- 验收命令输出贴进 PR；标记 `manual-check` 的卡只能提交"待人工验收"
- 每 PR 对应一个任务卡；不混改
- `docs/` 只收录本项目自身的设计与决策文档，不引入外部来源材料

## 已知坑
- libvips 子进程冷启动 ~5s → 走进程池预热
- `.workflow` 修改后 Finder 不立即刷新 → `killall pbs`
- 非 ASCII 的 .app 包名会坑 DMG/updater → 包名固定 `Yali.app`，中文显示走 CFBundleDisplayName 本地化
- **Tauri 2 没有 `capabilities/default.json` 时前端 `listen()` 被 ACL 拒绝**，事件静默失效（T-013 实测）
- **NSPasteboard 只能在主线程访问**；`run_on_main_thread` 的回调不要在 `setup` 里 `block_on` 等
  （主线程正被 setup 占着 → 死锁），要么在异步任务里等，要么改成事件驱动
- 剪贴板要比 `changeCount` 再比内容摘要：changeCount 变了不等于内容变了（同一张图复制两次）
- `before*Command` 的 cwd 是 `src-tauri` → 钩子写 `pnpm --dir ../ui …`
- 环境里 `CI=1` 会让 cargo-tauri 报 `invalid value '1' for '--ci'` → 整包用 `env -u CI`
- tauri-bundler 会给不认识的扩展名（avif/jxl）兜底 `public.image` → `src-tauri/Info.plist` 精确覆盖，
  `scripts/bundle-check.sh` 断言 UTI 与 `tauri.conf.json` 不漂移
- `tauri::generate_context!` 编译期需要 `src-tauri/icons/icon.png`（bundle.active=false 也要）
- `frontendDist`（`ui/dist`）在**非 dev** 构建时是硬校验：目录不存在直接 panic。`src-tauri/build.rs` 已兜占位页，但提交前务必跑一次 `pnpm --dir ui build` 让产物是真前端
