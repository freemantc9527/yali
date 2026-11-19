# 鸭梨 Yali

让图片没有压力。Yali 是 macOS 本地图片压缩与转换工具，提供 Finder 快速操作、桌面工作台和 CLI，共用同一套预设。

## 当前可用

- 压缩与转换：JPEG、PNG、WebP、AVIF、JPEG XL、HEIC、GIF 等，支持目标大小、负优化拦截和撤销清单。
- 桌面工作台：批量队列、本批参数、预设管理、应用设置、图片对比与 SSIM。
- 内置四个预设：日常压缩 / 转 JPG / 缩小尺寸 / 无损归档。缩小尺寸默认最长边 1024px，锁定宽高比、不放大小图；字号滑杆提供 80 / 100 / 120 / 150 / 180 / 200% 六档，默认 100%。
- 系统入口：Finder「压一下 / 转 JPG」、Dock / 打开方式投递、剪贴板监听、监视文件夹和菜单栏托盘。

响应式工作台已实现，原生交互仍待[人工验收](docs/tasks/T-021-responsive-workbench.md)。

## 准备环境

需要 macOS 14+、Command Line Tools、Rust、Node.js、pnpm、Python 3 和 Tauri 2 CLI，无需完整 Xcode。安装细节见 [TOOLCHAIN](docs/TOOLCHAIN.md)。

在仓库根目录安装项目依赖：

```bash
brew bundle --file "Brewfile"
pnpm --dir ui install --frozen-lockfile
```

引擎通过 Homebrew 安装，当前应用包尚未实现完整引擎分发。在新机器使用前也需要安装引擎，可用 `cargo run -p yali-cli -- doctor` 检查。

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

脚本构建真实前端与 release 应用，再校验 bundle 标识和图片 UTI；任一步失败即返回非零。默认产物为 `target/release/bundle/macos/Yali.app`，设置 `CARGO_TARGET_DIR` 时跟随该目录。脚本不安装应用，不执行公证或发布。

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
