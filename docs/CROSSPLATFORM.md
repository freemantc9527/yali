# CROSSPLATFORM · 跨平台方向备忘

顺序为 macOS → Linux → Windows，尚未排期或验证。Rust / React 有复用基础，但当前 core 依赖 sips 和 Homebrew 路径，不能宣称零平台代码或给出未经测量的复用比例。

| 范围 | 移植需要解决的问题 |
|---|---|
| 编码与解码 | 替换 sips / ImageIO 路径，准备各平台引擎与尺寸探测；HEIC 需重新评估 libheif / HEVC 许可与分发 |
| 右键入口 | Linux 可评估 Nautilus / KDE 服务脚本，Windows 可先评估打开方式 / 拖入，再做上下文菜单 |
| 系统集成 | 剪贴板、watch、托盘、通知分别验证；不能只因库跨平台就视为行为一致 |
| 智能能力 | macOS Vision 尚未落地；Linux / Windows 的 OCR / 抠图替代需另行选型与许可核查 |
| 分发与 CI | [T-028](tasks/T-028-macos-release.md) 实施 macOS 双架构构建与草稿流程，云端待验；Linux 包格式、Windows 引擎构建与跨系统测试矩阵仍待任务卡 |

`yali-integration` 仅预注册，尚未创建。真正启动移植时再将平台差异收口到接口，明确引擎发现和数据路径策略；不为未来平台提前建立抽象层，也不提供未验证的安装说明。
