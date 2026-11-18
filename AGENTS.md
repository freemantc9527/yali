# Yali · Agent 规则

用简体中文回复。用户当前指令优先；未经明确要求，不创建分支、commit 或 push，不自动 pull。保留已有工作区修改；删除文件/目录或修改用户、系统配置前须明确确认。

## 按需读取

- 开始先读 `docs/STATE.md` 和当前任务卡，避免重探已确认的环境与验收事实。
- 产品规格：`docs/DESIGN.md`；UI：`docs/UI-REDESIGN.md`；流程：`docs/WORKFLOW.md`。
- 构建：`docs/TOOLCHAIN.md`；质量：`benchkit/README.md`；Finder：`workflows/README.md`。只读相关章节。
- 命名以 `docs/NAMING.md` 为准：bundle `app.yali`，CLI `yali`，环境变量前缀 `YALI_`。不提前创建任务卡尚未启用的 crate 或目录。

## CodeGraph 优先

本项目已初始化 `.codegraph/`。理解、定位或修改代码前，先用 `codegraph_explore`，`projectPath` 传仓库绝对路径，query 写具体文件、符号或问题；MCP 不可用时用 `codegraph explore "<问题或符号>"`。

返回的逐字源码视为已读，不重复读取；未覆盖的文件或片段再用 `rg` / 定向读取。索引缺失或查询失败可退回 `rg`，不自行初始化或重建索引。

## 实施边界

- 只做当前任务范围，UI 禁止无卡改样式；一卡一 PR。`docs/` 仅收录本项目文档。
- macOS 仅 CLT，无 Xcode；禁止 `swift test`、`xcodebuild`、SwiftUI 宏。Swift 仅限任务批准的 `helpers/` 小工具。
- 前端统一 pnpm，锁文件入库。引擎路径参照 `Brewfile`，不可假定 PATH 中 `cjpeg` 是 mozjpeg。
- 调用 C / Vision / vips API 前，先查本机 SDK 头文件或实测，不凭记忆编造接口。
- NSPasteboard 仅在主线程访问，setup 不同步等待主线程回调；剪贴板同时按 changeCount 与内容摘要去重。
- watch 必须忽略自身产物，并按 settleMs 与 (mtime, size) 去重，防止自激。
- workflow 安装会修改用户 Services；模板变更后按任务范围重新安装并刷新 pbs，执行前确认授权。

## 验证与交付

| 修改范围 | 必跑 |
|---|---|
| 内核 | `cargo test -p yali-core` |
| Tauri 壳层 | `cargo test -p yali-app` |
| UI | `pnpm --dir ui test`、`pnpm --dir ui build` |
| 整包 | `bash scripts/build.sh`（含真实前端与 bundle 校验） |
| 压缩质量 | 重建 CLI 后运行 `benchkit/bench.sh`，避免使用旧二进制 |

本地运行 `bash scripts/dev.sh`，浏览器演示加 `--web`。Tauri 钩子从 `src-tauri` 执行，使用 `pnpm --dir ../ui …`。

保留事件 capability、精确 UTI 覆盖和编译期图标。release 必须包含真实 `ui/dist`，不能用 build.rs 占位页作为验收产物。

金样只按体积比 / SSIM 劣化 >3% 判失败，level≥5 单代照片另须 SSIM≥0.95。耗时、内存、CPU 等仅作观察，不阻塞验收，不为波动改基线。

交付附实际命令结果；更新任务卡与 STATE。`manual-check` 未经用户亲验只能标「待人工验收」，不得把浏览器模拟或自动化推定为人工通过。性能观察单独记录工具、环境和测量边界。
