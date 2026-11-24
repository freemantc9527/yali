# GitHub macOS 发布

公开发布只接受 GitHub Actions 从固定提交构建的产物。本地 `bash scripts/build.sh` 生成的 App 只用于自用和验证，不直接上传 Release。

## 发布流程

1. 在开发分支完成范围确认、测试、完整 App 构建和人工验收记录。
2. 按 [VERSIONING](VERSIONING.md) 同步版本，运行 `python3 scripts/release.py check`。
3. 合入 `main` 后，先检查目标提交、工作区和远端状态。
4. 在获准后创建并推送匹配版本的标签。标签推送会触发发布工作流。
5. GitHub Actions 在 Apple Silicon 和 Intel runner 上分别运行测试、构建、隔离冒烟和质量金样。
6. 汇总任务核对两个架构来自同一提交，回读 DMG、构建记录和 SHA256SUMS，然后创建 Release。

当前工作流位于 `.github/workflows/release.yml`。手动运行只生成 Actions artifacts，不创建标签或公开 Release。

2026-11-23 两次完整运行约需 31 到 33 分钟，其中 Apple Silicon 约 15 分钟、Intel 约 31 分钟。依赖安装与首次编译较慢，耗时只用于估算。排查时按阶段或每隔数分钟读取一次结构化状态，无需逐分钟查询；发现失败后再读取日志。

## Windows ZIP 实验

`.github/workflows/windows-experiment.yml` 在 `dev` 分支修改相关 Rust、UI、脚本、schema 或样本时触发，也保留 `workflow_dispatch` 手动入口。手动入口在工作流进入默认分支后可从 GitHub 页面使用。

此流程使用 windows-2025 x64，先按 scripts/windows-engines.lock.json 下载校验上游归档，再通过 scripts/build-windows.ps1 构建 mozjpeg、pngquant、gifsicle、libwebp，保留 libvips 官方运行目录和 oxipng 官方程序。工具使用静态 CRT；官方 oxipng 所需的可再分发 CRT 从 runner 的 Visual Studio Redist 目录收集并附许可，禁止依赖系统已安装的 VC++ 掩盖缺件。mozjpeg 使用上游支持的非 SIMD 构建，参数与质量门槛保持一致，耗时只作观察。

准备完成后实际执行 Rust 和 UI 测试，构建 release CLI、真实前端与 GUI，再用 scripts/windows-runtime.py package 检查每个 EXE/DLL 的 x64 PE、普通/延迟导入依赖、包内路径和文件摘要。ZIP 解压到中文空格目录后，scripts/smoke-windows.py 使用独立临时用户目录和受限 PATH，验证六类输入、缺件负例、动画策略、原图安全、撤销及相同引擎版本/参数的 SSIM/体积比门槛。FFmpeg 只作为 runner 的质量工具，既不放进 ZIP，也不用于无 FFmpeg 的基本处理步骤。

只有检查全部成功才上传 ZIP、SHA256SUMS、构建/依赖/冒烟 JSON 与对应源码 ZIP，保留 14 天，不创建 Release。源码包保留固定上游归档、MXE 构建配方/补丁、pngquant 和 librsvg 的 Cargo 依赖；运行包保留第三方许可与来源。不得拆掉对应源码或许可单独分发二进制。

本轮脚本尚未在 Windows runner 执行，不把本地 macOS 自动化写成 Windows 构建成功。Windows 11 新账户/VM 的 GUI、选择/拖入、预设重启、结果打开/定位、视觉对比、控制台闪窗、WebView2 缺失及 SmartScreen 表现须另行记录。GUI 使用用户已安装的 WebView2 Evergreen，不静默修改系统组件。

本地完成后，先确认具体 diff/提交内容，再单独确认推送 dev 及其会触发的 Windows Actions 运行；main 合入、标签和公开 Release/增补已有资产均须分别授权。旧 macOS alpha 资产不因本任务自动改变。

## 构建检查

每个架构执行：

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

每个版本至少包含 Apple Silicon 和 Intel 两份 DMG、对应构建记录及 `SHA256SUMS`。DMG 内含自包含 App 和应用程序目录快捷方式。构建记录保存仓库、运行链接、版本、提交 SHA、架构、引擎目标和校验和。

当前 App 使用 ad-hoc 签名，不包含 Developer ID、公证或 updater。公开下载后的首次运行可能需要用户在 macOS“隐私与安全性”中允许。当前 macOS 是主平台，Windows 不进入本发布流水线，直到真实 Windows 构建和验收完成。

## 失败处理

任一架构失败都不会创建 Release。修复后从新的候选提交重跑，不覆盖已分发版本。上传或网络失败后先读取远端草稿、资产和校验和，再决定重试或准备新版本，不盲目重复创建。
