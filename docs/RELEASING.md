# GitHub macOS 与 Windows 发布

公开发布只接受 GitHub Actions 从固定提交构建的产物。本地 `bash scripts/build.sh` 生成的 App 只用于自用和验证，不直接上传 Release。

## 发布流程

1. 在开发分支完成范围确认、测试、完整 App 构建和人工验收记录。
2. 按 [VERSIONING](VERSIONING.md) 同步版本，运行 `python3 scripts/release.py check` 和 `python3 scripts/windows-runtime.py verify-sources --build-dir target`，预检全部依赖归档与 SHA-256。
3. 合入 `main` 后，先检查目标提交、工作区和远端状态。
4. 在获准后创建并推送匹配版本的标签。标签推送会触发发布工作流。
5. 标签准备任务先检查版本、标签位于 main、固定源码提交和全部下载哈希，再启动 Apple Silicon、Intel 和 Windows x64 测试、构建、隔离冒烟与质量检查。Windows 只接受同次发布传入的固定候选提交。
6. 汇总任务强制三种架构来自同一仓库、提交和 Actions 运行，回读两个 DMG、Windows ZIP 与对应源码，校验依赖/冒烟/质量报告，生成 10 行 SHA256SUMS。
7. 仅标签触发的发布任务创建草稿，上传 11 份资产并下载回读 SHA-256；核对标签未移动后公开。任一前置检查失败都阻止公开。

当前发布入口为 `.github/workflows/release.yml`，只监听 `push.tags`；dev/main 普通推送不创建运行，不提供 `workflow_dispatch`。Windows 工作流仅保留 `workflow_call`，由同一次标签发布调用。运行页底部按平台下载：

| 用途 | Artifact |
| --- | --- |
| Apple Silicon Mac | `desktop-macos-arm64` |
| Intel Mac | `desktop-macos-x86_64` |
| Windows x64 程序 | `desktop-windows-x86_64` |
| Windows 第三方对应源码 | `desktop-windows-corresponding-sources` |
| 校验和与发布说明 | `desktop-release-metadata` |

各平台的程序单独下载。汇总任务验证资产后只上传校验和与发布说明，发布任务从同次运行读取原始分平台资产并核对校验和。公开发布仍须确认 main 合入、标签推送和发布范围，不得覆盖已有版本。不再执行 dev 构建或手动演练来准备发布；本地预检通过后，只从 main 上的版本标签启动完整流水线。

历史分支构建和手动演练记录只代表当时的验证方式；当前流程不再使用这些入口。

## Windows ZIP 实验

`.github/workflows/windows-experiment.yml` 只保留 `workflow_call`，候选 `ref` 必填。构建任务检查调用事件是版本标签 push，且传入提交与标签解析出的提交一致，不接受 dev/main 或手动运行。

此流程使用 windows-2025 x64，先按 scripts/windows-engines.lock.json 下载校验上游归档，再通过 scripts/build-windows.ps1 构建 mozjpeg、pngquant、gifsicle、libwebp，保留 libvips 官方运行目录和 oxipng 官方程序。工具使用静态 CRT；官方 oxipng 所需的可再分发 CRT 从 runner 的 Visual Studio Redist 目录收集并附许可，禁止依赖系统已安装的 VC++ 掩盖缺件。mozjpeg 使用上游支持的非 SIMD 构建，参数与质量门槛保持一致，耗时只作观察。

准备完成后实际执行 Rust 和 UI 测试，构建 release CLI、真实前端与 GUI，再用 scripts/windows-runtime.py package 检查每个 EXE/DLL 的 x64 PE、普通/延迟导入依赖、包内路径和文件摘要。ZIP 解压到中文空格目录后，scripts/smoke-windows.py 使用独立临时用户目录和受限 PATH，验证六类输入、缺件负例、动画策略、原图安全、撤销及相同引擎版本/参数的 SSIM/体积比门槛。FFmpeg 只作为 runner 的质量工具，既不放进 ZIP，也不用于无 FFmpeg 的基本处理步骤。

只有检查全部成功才生成发布清单，并把程序 ZIP（附清单及依赖/冒烟/质量 JSON）与对应源码 ZIP 分别上传，保留 14 天。不提供单独实验运行入口。源码包保留固定上游归档、MXE 构建配方/补丁、pngquant 和 librsvg 的 Cargo 依赖；运行包保留第三方许可与来源。用户运行程序只需下载程序 ZIP；分发方须同时提供对应源码和许可。

含引擎的 Windows 构建已在 [Actions 45321363937](https://github.com/freemantc9527/yali/actions/runs/45321363937) 成功，下载回读确认二进制、对应源码和 SHA-256 匹配。Windows 11 新账户/VM 的 GUI、选择/拖入、预设重启、结果打开/定位、视觉对比、控制台闪窗、WebView2 缺失及 SmartScreen 表现须另行记录。GUI 使用用户已安装的 WebView2 Evergreen，不静默修改系统组件。

本地完成后按授权推送源码并合入 main；普通分支推送不会运行 Actions。确认版本标签后推送标签，三架构检查与公开 Release 在同次运行完成；不修改旧标签和旧资产。

## 构建检查

macOS 每个架构执行：

```bash
cargo test --locked -p yali-core -p yali-cli -p yali-app -- --test-threads=1
pnpm --dir ui test
bash scripts/build.sh
python3 scripts/smoke-bundle.py
python3 -m unittest discover -s scripts -p 'test_*.py' -v
python3 scripts/release.py check
```

压缩质量变更还要用本次构建出的 CLI 运行 `benchkit/bench.sh`。耗时和系统资源只作观察，体积比和 SSIM 质量门槛才阻断发布。

## 产物与限制

每个新版本必须包含 11 份公开资产：Apple Silicon 和 Intel 两份 DMG、两份 macOS 构建记录，Windows x64 实验 ZIP、对应源码 ZIP、Windows 发布清单及三份依赖/无 FFmpeg 冒烟/质量报告，以及 `SHA256SUMS`。DMG 内含自包含 App 和应用程序目录快捷方式。构建记录保存仓库、运行链接、版本、提交 SHA、架构、引擎目标和校验和；Windows 清单还绑定所有配套报告与对应源码的摘要。

当前 macOS App 使用 ad-hoc 签名，不包含 Developer ID、公证或 updater。公开下载后的首次运行可能需要用户在 macOS“隐私与安全性”中允许。Windows 保留实验标记和 GUI 待验声明，不以 Server runner 的自动化代替 Windows 11 桌面验收。`v0.3.0-alpha` 已在[标签运行 45782095121](https://github.com/freemantc9527/yali/actions/runs/45782095121) 完成三架构验证并公开 11 份资产；本地公开下载回读的 10 条 SHA-256、两份 DMG 与 Windows ZIP/对应源码/报告均通过。程序按平台独立下载，旧版 `v0.2.0-alpha` 与 `v0.1.0-alpha` 保留。该版本候选首次检查遇到上游复用版本标签，固定上一版相同源码的不可变提交后通过；正式标签保持不变。

## 失败处理

GitHub archive/codeload/tarball 自动生成的源码归档统一固定到完整 40 位提交 SHA；禁止分支、标签或短 SHA，下载后仍核对 SHA-256。正式 Release 下载资产使用固定 URL 与 SHA-256。校验失败后先与已验证的原归档比较文件内容、类型、权限和链接，再记录新归档 SHA-256 与来源证据；不得只替换哈希来接受标签下的新源码。

任一架构失败都不会创建 Release。修复后从新的候选提交重跑，不覆盖已分发版本。上传或网络失败后先读取远端草稿、资产和校验和，再决定重试或准备新版本，不盲目重复创建。
