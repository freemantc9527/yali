# T-028 · GitHub Actions macOS 双架构发布

status: 本地已实施并验证，待云端与人工验收（2026-11-19）
goal: 同一源码分别原生构建 Apple Silicon / Intel 自包含 App，汇总为可核验的 GitHub Release 草稿
manual-check: true

## 已批准范围

- 用户选择首批 macOS 双架构，暂不移植 Windows / Linux。
- 新增发布工作流、最小发布脚本及验证，复用现有构建、隔离运行与质量回归。
- 手动运行仅产出 Actions artifacts；推送版本标签时校验 main 来源与版本一致性，两架构通过后创建草稿。
- 保留 ad-hoc 签名，不引入证书、公证或 updater；最低系统按各架构包内引擎实测记录。
- 初轮只准备本地可审查结果；用户后续明确授权先提交、合并到 main 并推送远端。该授权不包含标签创建、远端工作流运行或公开发布，不修改仓库设置。

## 设计

决策见 [ADR-007](../decisions/ADR-007-github-macos-release.md)，操作见 [RELEASING](../RELEASING.md)。本轮涉及超过 8 个文件，主要为任务、决策、操作文档及入口；产品内核、UI 与系统安装逻辑不改。

- 固定 `macos-15`（arm64）与 `macos-15-intel`（x86_64），运行时再核对实际架构；不做只合并主程序的 universal 包。
- 构建任务只读仓库，仅汇总草稿的任务拥有 `contents: write`；Action 固定到经官方仓库核对的完整 SHA。
- 使用最终 App 的 zip，保留符号链接、权限与签名；每架构记录源码 SHA、运行时目标、最低系统、工具版本与 zip SHA-256。
- 汇总严格要求两架构、同版本和同源码，核对归档内元数据与校验和；默认不覆盖已有版本。

## 验收

- 发布脚本测试覆盖版本不一致、非法标签、缺少架构、错配源码、归档篡改与路径越界。
- 工作流通过 actionlint；本机原生构建、测试、金样与真实 zip 打包 / 回读通过。
- GitHub 双架构完整运行、Intel 真机交互及公开下载后的 Gatekeeper 行为待真实验证，不以本地检查代替。

## 实际记录

- 只读核对：仓库为 public、默认分支 main、Actions enabled，默认 token 只读；当前远端工作流及 Release 列表均为空。
- 当前工作区保留 T-027 的 5 个文档变更；不将其混作已提交或已发布状态。
- 官方 GitHub runner 文档确认 `macos-15` 为 arm64、`macos-15-intel` 为 Intel；官方 Action 仓库的 tag / commit API 核对完整 SHA。仅查询元数据，未改仓库权限或设置。
- 从 `4ccbf3f` 导出独立候选目录，仅加入本卡的发布脚本、工作流及 Python 缓存忽略规则；产品源码保持该基线，复用本地依赖与构建缓存。候选位于忽略的 target 内，不建分支或提交。
- `python3 -m unittest discover -s scripts -p 'test_release.py' -v`：10 passed，含已知正确双架构样本与明确错误输入；无原生二进制执行的合成归档仅用于验证汇总拒绝规则。
- 校验下载的 actionlint `1.7.12` 官方 SHA-256 后运行 `actionlint .github/workflows/release.yml`：退出码 0。`python3 scripts/release.py check` 输出 version=0.0.1、commit=4ccbf3fef893faeae0a6292be49423bdacd3a89e、prerelease=false。
- 候选运行 `cargo test --locked --offline -p yali-core -p yali-cli -p yali-app -- --test-threads=1`（独立 YALI_HOME、复用 CARGO_TARGET_DIR）：95 passed，9 suites；`pnpm --dir ui test`：72 passed，11 files。
- 候选运行 `bash scripts/build.sh`（显式 CARGO_TARGET_DIR）：真实 UI 构建、App 与 CLI 通过；13 个可执行文件、97 个动态库 / 插件，111 个 Mach-O 依赖及 ad-hoc 签名检查通过，10 类图片 / 10 个 UTI 通过。
- `release.py package --arch arm64` 打包真实 App 并解压回读，再次运行 bundle-check 通过；zip SHA-256 为 `0f5fcae7f44ad5be80f647ff7514ae0f87d74be79aea28ac85851c20a3bc6157`。该产物只用于本地验证，记录 sourceDirty=true，不能作为干净发布候选汇总。
- 本机环境为 macOS 27.2.1 / arm64、Rust 1.100.0、Tauri CLI 2.12.5、pnpm 12.21.0，实际打包记录 Node 26.13.1；云端指定 Node 22 / Python 3.13，未声称本机覆盖其环境。此处无性能测量结论。
- `python3 scripts/smoke-bundle.py --app <本次最终 App>`：沙箱内首次因禁止嵌套 sandbox-exec 失败；获准在沙箱外复验通过。验证了无 Homebrew / 源码 / 原账户数据时六张金样、四预设、WebP / AVIF / JXL、TIFF / BMP、两条真实临时 Automator workflow 和包内 ffmpeg。只使用临时 Home / Services，不安装到用户目录。
- `YALI_BIN=<本次 App 内 CLI> bash benchkit/bench.sh --profile default` 与 `--profile jpg`：沙箱内均在 HEIC sips 写出时报错；获准沙箱外复验，两档均 6 张金样通过，体积比 / SSIM 在 3% 门槛内。jpg 档 HEIC 耗时约 +4.0% 仅观察，不改基线。
- `git diff --check` 通过；新增操作文档、ADR 与本卡标点检查通过。最终检查包含前轮文档共 15 个变更文件，99 个本地 Markdown 链接均存在，Python 语法通过，产品版本保持 0.0.1。最终工作区恰好这些文件，HEAD 未改变，无提交或远端写操作。
- 草稿创建前再次读取远端标签对应 SHA，并分页检查所有 Release；本地 jq 正反例验证空列表 / 其他版本允许，同标签草稿 / 公开版本均拒绝。最终 actionlint 与差异检查仍通过。

## 待验

- [ ] 授权交付工作流后，在 GitHub 手动运行两架构；核对实际 runner、工具链、依赖、包内最低系统和隔离运行日志。
- [ ] 发布准备卡统一产品版本并人工核对后，推送指定 main 标签，验证 Release 草稿包含 5 个资产及下载回读。
- [ ] Intel 真机交互、公开下载后的系统安全提示，以及 STATE 中本版范围内的人工验收。
- 不将以上待验写为通过，不声称已公开任何版本。

## 用户补充：发布包只来自 GitHub

- 本地打包仅供自用和本地验证，不用于发布。规范、README 与 ADR 已同步；云端构建复用同一打包脚本，但不接收本地 App 或 zip 作为输入。
- 构建记录增加 `buildOrigin`、`repository`、`runId`、`runUrl`；本地标记 `local-validation`。发布汇总要求当前处于 GitHub Actions，且两份产物属于当前仓库、当前运行及当前提交；即使本地源码干净也拒绝本地包。
- 本轮发布脚本 14 项测试通过，新增已知错误用例覆盖本地包、本地汇总、跨运行 / 仓库产物、GitHub 上下文与源码不符；actionlint 与 `git diff --check` 退出码 0。未修改产品逻辑，不重新打本地 App 作为发布证据。
- 只读复核远端：Actions workflows total_count=0，Release 列表为空。当前只能确认工作流配置具备双架构构建、上传和回读能力，尚不能确认云端实跑成功或公开下载可用。
- 草稿不公开；公开发布后才在 Releases / Assets 向用户提供 arm64 与 x86_64 两份 zip。首个公开版本的验收必须包含两架构云端成功、远端资产下载校验及公开页面可见性。

## 源码交付复核

- 用户明确要求提交当前变更、合并到 main 并推送远端；提交包含 T-027 / T-028 的规范、发布脚本、测试和工作流，不改产品版本，不创建 Release。
- 交付前 `git ls-remote origin refs/heads/main refs/heads/dev`：远端 main 为 `e1bfc4eeef8118baeacbf4205dede5e88a2856cd`，与本地 main 相同，远端无 dev。dev 已有 5 个本地提交，本次合并会一并纳入 main；不改写这些历史。
- 本轮重新运行发布脚本 14 项测试、actionlint 与 `git diff --check`，均通过。产品测试、整包、隔离运行与金样沿用前述同一产品源码的实测记录；本轮没有把本地 App 用于发布。
- 只推送 main 的明确 ref，采用快进合入，不强推、不推标签。GitHub 工作流只响应手动运行和版本标签，普通 main 推送不会触发双架构构建。
