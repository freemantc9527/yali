# ADR-006 · 自包含个人学习版 App

> 状态：接受 · 日期 2026-11-19 · 决策人：用户批准

## 背景

用户要求只分享构建后的 App，即可在无 Homebrew、无源码目录的账户中使用 GUI、CLI 和两条 Finder 快速操作。暂不承担 Apple Developer 证书成本。

## 决策

- GUI 与 CLI 共用 `yali-core`，CLI 和图片引擎由 Tauri `externalBin` 放入 `Contents/MacOS/`；动态库与 libvips 图片插件放入 `Contents/Frameworks/`。
- 构建脚本递归收集 Mach-O 动态依赖，改成包内 `@loader_path` 引用并移除外部 RPATH。图片处理引擎在包内运行时不查 Homebrew、PATH 或 `YALI_ENGINES_PATH`，缺件直接失败。2026-11-20 的补充：FFmpeg 不再打包，仅作为可选外部 SSIM 与解码校验工具查找，缺失时仍能处理图片和视觉对比；其状态及路径只保留在会话内，可在设置页重新检查。
- `Resources/yali-runtime.json` 记录引擎、动态库、架构和实际最低 macOS；从所有引擎与动态库的部署目标取最大值，不通过改版本号伪造旧系统兼容。运行时清单是完整包标记。
- 配置、预设、日志、缓存、撤销记录继续使用 `~/.yali/`，`YALI_HOME` 可隔离。`~/.yali/bin/yali` 是指向 App 内 CLI 的稳定符号链接；不修改 shell 的 PATH。
- Finder workflow 必须安装到 `~/Library/Services/`。设置页与 CLI 共用安装实现，不依赖 Python，不改写 `pbs` 的私有偏好，不重启 Finder；使用公开 `NSUpdateDynamicServices()` 刷新，菜单被系统禁用时由用户启用。此条取代旧 workflow 文档的偏好表写入策略。
- 安装仅接受 `/Applications/` 或 `~/Applications/` 中的完整 App，避免登记下载隔离或挂载镜像中的临时路径；移动后重新打开 App 可修复已有 CLI 链接。
- 只生成 ad-hoc 签名，不签 Developer ID、不公证、不更改系统隔离属性。用户按系统提示允许首次运行；运行后不修改 App 自身。
- 构建输入不作为分发产物，分享包只包含最终 App。Node/SVGO 不随包分发；产品声明的十类位图格式保留，右键不再声明未分发的 SVG 优化能力。

## 后果

- 承担：包体积增加，插件与动态依赖必须一并验收；当前本机引擎实际要求 macOS 27.2，首批产物限 Apple Silicon。旧系统兼容需要针对旧部署目标重新构建依赖，不由本卡承诺。
- 放弃：依赖开发仓库路径、接收者自行装 Homebrew、用户目录中另一份完整运行时、后台常驻安装服务、私有系统偏好改写。
- 回滚：设置页可卸载本应用两条 workflow；配置、图片与稳定 CLI 入口保留。恢复旧 App 不需要迁移设置 schema。未实施系统级安装、更新或发布。

## Windows 实验补充（2026-11-23）

Windows ZIP 采用独立布局与 schemaVersion=1 的运行时清单，保留同一子进程引擎架构，不复用 macOS Contents 根。macOS Services 的根判断保持 macOS 专用。工具仅以包内绝对路径运行，校验工具映射、相对路径、x64 架构、文件摘要与构建时依赖闭包；摘要用于完整性检查，不证明来源可信。

ZIP 不含 Node、Python、完整 SDK、FFmpeg 或完整 WebView2。工具按锁文件准备；C 工具静态链接 CRT，官方 MSVC 工具需要的 CRT 在许可允许范围内随包保留。libvips 的目录及插件结构保持上游布局；第三方许可证、对应源码、构建配方和 Cargo 依赖随同 artifact 分发。2026-11-24 Windows runner 已通过含引擎构建、Rust/UI 测试、PE/DLL 和最终 ZIP 真实处理/质量检查；Windows 11 GUI 仍待人工验收。
