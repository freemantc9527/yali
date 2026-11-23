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

`.github/workflows/windows-experiment.yml` 在 `dev` 分支修改该文件时触发，也保留 `workflow_dispatch` 手动入口。手动入口在工作流进入默认分支后可从 GitHub 页面使用。

此流程使用 Windows x86_64 runner 编译 Rust 测试、运行 UI 测试、构建真实前端与两个程序，再将 `Yali-workbench.exe`、`yali.exe`、许可证、实验说明和构建记录组成 ZIP。解压回读后执行 CLI 版本检查，上传 ZIP 与 SHA-256 为保留 14 天的 Actions artifact，不创建 Release。

Rust 测试只编译，不等同于执行通过。ZIP 不含图片引擎，GUI 需要系统安装 Microsoft Edge WebView2 Runtime；免安装指程序解压后运行，配置和缓存仍使用常规数据目录。GUI 启动、图片处理与运行时依赖须在真实 Windows 环境另行验证。

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
