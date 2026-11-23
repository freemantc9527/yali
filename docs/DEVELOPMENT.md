# 开发指南

鸭梨是 Rust 工作区、Tauri 壳层和 React UI 的组合项目。macOS 是主要开发平台，工程只要求 Command Line Tools，不使用完整 Xcode。

## 环境

固定版本和本机依赖见 [TOOLCHAIN](TOOLCHAIN.md) 与 `Brewfile`。准备环境后执行：

```bash
brew bundle --file "Brewfile"
pnpm --dir ui install --frozen-lockfile
cargo run -p yali-cli -- doctor
```

`doctor` 用来检查图片引擎和必要工具。发布包会把图片处理运行时放入 App，开发机上的 Homebrew 依赖不会随源码分发给用户。

## 运行

```bash
bash scripts/dev.sh
bash scripts/dev.sh --web
```

桌面模式通过 Tauri 启动真实系统入口。`--web` 只用于浏览器调试，数据和处理结果是降级实现，不能替代 Finder 或原生窗口验收。

CLI 可以直接运行：

```bash
cargo run -p yali-cli -- profiles list
cargo run -p yali-cli -- profiles show default
cargo run -p yali-cli -- compress "/path/to/photo.jpg" --profile default
```

开发时可以设置 `YALI_HOME` 把配置、缓存、撤销记录和日志隔离到临时目录：

```bash
YALI_HOME="/tmp/yali-dev" cargo run -p yali-cli -- profiles list
```

## 代码布局

- `crates/yali-core`：Profile、数据目录、图片处理、缓存、撤销和系统入口的共享内核。
- `crates/yali-cli`：命令行入口，负责参数解析、脚本输出和与 App 的通知协作。
- `src-tauri`：Tauri 应用、系统事件、菜单栏、通知、Services 与内核命令桥接。
- `ui`：React + Vite + Tailwind 的工作台和设置界面。
- `workflows`：Finder Services 模板和安装说明。
- `benchkit`：质量金样、基线和压缩回归脚本。
- `scripts`：开发、构建、冒烟和发布检查脚本。
- `.tasks`：被 Git 忽略的开发任务卡和过程记录。

## 验证

按改动范围执行检查：

```bash
cargo test --locked -p yali-core
cargo test --locked -p yali-cli
cargo test --locked -p yali-app
pnpm --dir ui test
pnpm --dir ui build
bash scripts/build.sh
```

完整 App 需要通过 `scripts/build.sh` 构建，不能用只含占位页面的开发产物代替。压缩算法或默认预设发生变化时，先重建 CLI，再运行 `benchkit/bench.sh`，按体积比和 SSIM 检查金样。

## 定位代码

仓库包含 `.codegraph/` 时，先用：

```bash
codegraph explore "描述要查找的符号、文件或调用关系"
```

返回的逐字源码足够回答问题时，不重复用 `rg` 读取同一段。CodeGraph 未覆盖的文件再用 `rg` 和定向读取。

## 开发数据与清理

配置、预设、撤销记录、缓存和日志统一从 `yali_core::data` 解析数据根。测试或调试结束后只清理自己创建的临时目录；不要删除用户的 `~/.yali/`，不要绕过缓存租约直接清理活动预览。

## 常见边界

- 没有外部 FFmpeg 时，图片处理和视觉对比仍可用，SSIM 分数隐藏。
- Finder、通知、托盘、剪贴板、watch 和原生窗口的自动化结果不能代替人工验收。
- Windows 代码处于实验阶段，本机未验证交叉编译、打包和系统集成。
- 开发机产物只用于自用和验证，公开 Release 必须由 GitHub Actions 从固定提交构建。
