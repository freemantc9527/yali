# GitHub macOS 双架构发布

工作流：[release.yml](../.github/workflows/release.yml)。范围与选择见 [T-028](tasks/T-028-macos-release.md) 和 [ADR-007](decisions/ADR-007-github-macos-release.md)，版本规则见 [VERSIONING](VERSIONING.md)。

本地打包仅供自用与本地验证，不用于发布，不上传到 Release。所有发布包必须由本仓库 GitHub Actions 从同一提交分别原生构建；发布汇总只接受本次云端运行的两个架构产物。

## 运行方式

| 入口 | 行为 |
|---|---|
| Actions → macOS release → Run workflow | 构建所选 ref 的 Apple Silicon / Intel 包，仅保存 Actions artifacts，不创建标签或 Release |
| 推送匹配产品版本的 `v*` 标签 | 标签必须指向 main 历史；双架构全部通过后自动创建 Release 草稿 |
| 在 GitHub 打开草稿并公开 | 维护者核对本版人工验收、已知问题、用户可见变化及资产后操作 |

草稿及其资产仅对有相应仓库权限的人可见；公开后，用户才能在 [Releases 页](https://github.com/freemantc9527/yali/releases) 的 Assets 中下载 Apple Silicon（arm64）或 Intel（x86_64）zip。Actions artifacts 属于构建验证入口，不替代公开 Release 下载。

首次手动运行前，工作流须已进入远端默认分支。提交、推送、合入、标签与公开发布仍按用户明确授权执行；本地写好 YAML 不代表工作流已运行。

当前实际版本为 `0.0.1`。准备 `0.1.0-alpha.1` 时，先开发布卡，统一修改 workspace、Tauri、UI 与 Cargo.lock 的产品版本并完成检查。`python3 scripts/release.py check --tag "v0.1.0-alpha.1"` 在版本尚未同步时会失败，不会替用户改版本或创建标签。

## 构建与产物

```text
版本 / 标签 / main 来源检查
  → macos-15：原生 arm64 测试、构建、隔离运行、质量金样
  → macos-15-intel：原生 x86_64 测试、构建、隔离运行、质量金样
  → 核对同版本与同源码 → Release 草稿 → 人工核对后公开
```

两个构建任务并行，任一失败都不会进入草稿任务。各自上传 artifacts 的保留期为 14 天；Release 资产另行保存。

每个版本共 5 个下载资产：两份 `Yali_<版本>_macos_<架构>.zip`、两份同名 `.json` 构建记录，以及 `SHA256SUMS`。zip 只包含最终自包含 App；使用 `ditto` 保留符号链接与权限，归档后解压回读，再检查动态依赖和签名。

构建记录包含构建来源、GitHub 仓库与运行链接、产品版本、源码完整 SHA、工作区是否有未提交修改、CLI / 引擎目标架构、包内原生版本字段、实际最低系统、工具与引擎版本、zip SHA-256。本地包标记为 `local-validation`；汇总只在 GitHub Actions 中运行，并拒绝本地包、其他仓库或运行的包，以及缺包、混用源码、未提交修改、错误架构、元数据冲突或校验失败。上传后下载 Release 资产，再核对 SHA256SUMS 和远端草稿状态。

首次运行前，GitHub 双架构链路与 Intel 支持均应标为待验证。构建机为 macOS 15 不代表产物保证支持 macOS 15；引擎实际部署目标更高时如实记录，并由运行验证暴露不兼容。不得只调低 Info.plist 或忽略失败来通过构建。

## 检查与依赖

固定 Rust `1.100.0`、pnpm `12.21.0`、Tauri CLI `2.12.5`；Node 使用 22 系列，Python 使用 3.13 系列，Homebrew 依赖以本次 Brewfile 安装及包内记录为准。Action 使用完整提交 SHA。尚未锁定全部构建输入，因此不承诺逐位可复现。

每架构运行：

```bash
cargo test --locked -p yali-core -p yali-cli -p yali-app -- --test-threads=1
pnpm --dir ui test
bash scripts/build.sh
python3 scripts/smoke-bundle.py
```

`scripts/build.sh` 包含真实 UI 构建、引擎打包、动态依赖、签名和 UTI 校验。随后 `benchkit/bench.sh` 分别跑 default / jpg，使用本次最终 App 中的 CLI；质量门槛沿用原规则，耗时不阻断。

发布脚本可单独检查：

```bash
python3 -m unittest discover -s scripts -p 'test_release.py' -v
python3 scripts/release.py check
```

流水线只调用现有 CLT 兼容构建命令，不使用 `xcodebuild` 或 `swift test`。原生人工验收仍留在对应任务卡，自动检查不将其标记完成。

## 权限与签名

仓库 Actions 须启用，且允许该工作流声明 `contents: write`。本仓库已只读核对为 public、Actions enabled，默认 token 为只读；工作流只给草稿任务写权限。

唯一使用的令牌是 GitHub 自动提供的 `GITHUB_TOKEN`，无需配置个人 PAT 或 Apple 凭据。构建任务不持有写令牌，不保留 checkout 凭据。当前只生成 ad-hoc 签名，不包含 Developer ID、公证或应用内 updater；公开下载后的首次运行仍遵循 macOS 安全提示。

## 失败与重试

构建或验证失败时，查看失败架构的日志，修复后重试；不自动修改基线、版本、标签或系统声明。已分发产物发生变化时使用新版本，不能通过重跑覆盖。

草稿任务不使用资产覆盖选项；同标签 Release 已存在时失败。网络或上传失败可能留下部分草稿，先核对远端资产及 SHA，再决定补传同一份已验证资产或准备新版本，不盲目重复创建，不自动删除。

公开前补齐草稿中的本版变化与待验说明。Alpha / Beta / RC 后缀自动设置 GitHub 的 pre-release 标记，无后缀版本也先生成草稿；公开是独立操作。

## 官方依据

- [GitHub runner 规格](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)：`macos-15` 为 arm64，`macos-15-intel` 为 Intel。
- [GitHub Actions 数据传递](https://docs.github.com/en/actions/how-tos/writing-workflows/choosing-what-your-workflow-does/storing-and-sharing-data-from-a-workflow)：构建任务上传、草稿任务下载 artifacts。
- [Tauri macOS 签名](https://v2.tauri.app/distribute/sign/macos/)：ad-hoc 与 Developer ID / 公证的能力边界。
