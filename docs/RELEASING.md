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
