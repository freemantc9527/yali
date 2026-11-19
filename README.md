# 鸭梨 Yali

让图片没有压力。Yali 是 macOS 本地图片压缩与转换工具，提供 Finder 快速操作、桌面工作台和 CLI，共用同一套预设。

## 当前可用

- 压缩与转换：JPEG、PNG、WebP、AVIF、JPEG XL、HEIC、GIF 等，支持目标大小、负优化拦截和撤销清单。
- 桌面工作台：批量队列、本批参数、预设管理、应用设置、图片对比与 SSIM。
- 内置四个预设：日常压缩 / 转 JPG / 缩小尺寸 / 无损归档。缩小尺寸默认最长边 1024px，锁定宽高比、不放大小图；字号滑杆提供 80 / 100 / 120 / 150 / 180 / 200% 六档，默认 100%。
- 系统入口：Finder「压一下 / 转 JPG」、Dock / 打开方式投递、剪贴板监听、监视文件夹和菜单栏托盘。

响应式工作台已实现，原生交互仍待[人工验收](docs/tasks/T-021-responsive-workbench.md)。

## 使用构建好的 App

将 `Yali.app` 复制到 `/Applications/` 或 `~/Applications/` 后打开，无需 Homebrew、源码、Node 或 Python。右键菜单在 **设置 → 自动化 → Finder 右键操作** 中安装、修复或卸载；若系统未显示菜单，请在系统设置的“服务”或“扩展”中启用鸭梨。

安装右键操作后，CLI 可通过 `~/.yali/bin/yali` 使用，也可直接调用 App 内的 `Contents/MacOS/yali`。稳定入口不自动加入 PATH；覆盖升级无需重新配置，移动 App 后重新打开一次即可修复入口。

个人学习版使用 ad-hoc 签名，没有 Developer ID 或公证。首次打开若被系统拦截，请按 macOS 系统提示在“隐私与安全性”中允许。当前本机产物为 Apple Silicon，最低 macOS 27.2，来自所打包引擎的实际部署目标；低版本系统未经验证。

## 开发环境

需要 macOS 14+、Command Line Tools、Rust、Node.js、pnpm、Python 3 和 Tauri 2 CLI，无需完整 Xcode。安装细节见 [TOOLCHAIN](docs/TOOLCHAIN.md)。

在仓库根目录安装项目依赖：

```bash
brew bundle --file "Brewfile"
pnpm --dir ui install --frozen-lockfile
```

开发机通过 Homebrew 安装引擎，可用 `cargo run -p yali-cli -- doctor` 检查。`scripts/build.sh` 把运行所需工具和完整动态依赖装入 App，接收者无需安装这些开发依赖。

## 本地运行

```bash
bash scripts/dev.sh          # 原生桌面应用，前后端热更新
bash scripts/dev.sh --web    # 浏览器演示：http://localhost:5173
```

浏览器模式仅模拟处理；真实压缩和系统入口请使用桌面应用。脚本从仓库外也可用绝对路径调用。

CLI 可单独使用：

```bash
cargo run -p yali-cli -- profiles show default
cargo run -p yali-cli -- compress "/path/to/photo.jpg" --profile default
```

预设、设置、撤销记录和缓存统一位于 `~/.yali/`；`YALI_HOME` 可重定向整个数据根。首次启动会复制旧版应用目录内的缺失配置，保留旧文件。压缩后的图片仍保存到预设指定的位置。

## 打包构建

```bash
bash scripts/build.sh
```

脚本构建 CLI、真实前端与 release 应用，收集并重定位引擎和动态库、保留第三方许可证与构建来源、生成 ad-hoc 签名，最后校验 bundle 标识、图片 UTI、包内全部动态依赖和签名。任一步失败即返回非零。默认产物为 `target/release/bundle/macos/Yali.app`，设置 `CARGO_TARGET_DIR` 时跟随该目录；不要绕过脚本把裸 `cargo tauri build` 产物当作分享包。脚本不安装应用，不执行公证或发布。

`python3 scripts/smoke-bundle.py` 复制最终 App 到专用临时目录，以隔离 HOME / 数据根、最简 PATH 和禁止读取 Homebrew / 源码 / 原账户的沙盒，验证四个预设、实际格式转换、强隔离的 workflow shell 节点及正常系统上下文中的两条完整 Automator workflow（macOS 禁止沙盒进程使用 Automator.framework）。完成后清理测试副本；加 `--keep` 保留用于原生 GUI 验收。此自动验收不替代 Finder 菜单可见性与原生交互亲验。

## 验证与贡献

验证命令见 [AGENTS](AGENTS.md)，贡献流程见 [CONTRIBUTING](CONTRIBUTING.md)。压缩质量按 [benchkit](benchkit/README.md) 的体积比 / SSIM 门槛验收；耗时和系统资源只记录观察。

## 项目文档

| 文档 | 用途 |
|---|---|
| [STATE](docs/STATE.md) | 当前进度与人工待验项 |
| [DESIGN](docs/DESIGN.md) / [UI-REDESIGN](docs/UI-REDESIGN.md) | 产品规格 / 已确认的响应式 UI 方案 |
| [任务卡](docs/tasks/README.md) / [架构决策](docs/decisions/README.md) | 任务范围、验收证据与选型依据 |
| [TOOLCHAIN](docs/TOOLCHAIN.md) / [NAMING](docs/NAMING.md) | 构建约束与命名注册表 |
| [WORKFLOW](docs/WORKFLOW.md) | 贡献与验收制度 |

## 许可

[Apache-2.0](LICENSE)
