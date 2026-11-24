# 跨平台支持

macOS 是鸭梨的主平台，Finder、桌面工作台、CLI、自包含运行时和系统通知都优先在 macOS 上维护。其他平台先保证基本图片处理，再决定是否加入系统入口和完整格式支持。

## macOS

当前本机验证覆盖 Apple Silicon。发布流程计划分别构建 Apple Silicon 和 Intel 产物，但每个版本的实际最低系统、架构和格式范围以 Release 构建记录为准。当前图片引擎的实际最低部署目标为 macOS 27.2，低版本系统尚未验证。

## Windows

Windows 代码处于实验阶段。[首轮云端试构建](https://github.com/freemantc9527/yali/actions/runs/45133346294)已完成 x86_64 App / CLI 编译、78 项 UI 测试、ZIP 组装及解压后的 CLI 版本检查。首轮 ZIP 没有内嵌图片引擎。后续代码已补齐包内运行时、专用引擎构建和最终 ZIP 冒烟脚本，但本轮尚未执行 Windows 云端构建；WebView2、GUI 运行、动态库和格式兼容性仍待真实 Windows 验收。首个 Windows 版本只计划保证 JPEG、PNG、GIF、WebP、BMP、TIFF 等基本图片处理，不承诺 macOS 的 HEIC、AVIF、JPEG XL 和 Finder Services 能力。

| 范围 | 当前边界 |
| --- | --- |
| 图片处理 | 候选范围为 JPEG、PNG、GIF、WebP、BMP、TIFF 输入；JPEG、PNG、WebP 输出，GIF 默认保留动画；多页 TIFF、动画转静态和不保帧的重编码明确拒绝。Windows 实测待完成 |
| 系统入口 | 暂不承诺右键菜单、文件关联、剪贴板、watch 和通知 |
| 图形界面 | 只有在真实 WebView2 环境验证后才扩大支持范围 |
| 分发 | Windows ZIP 实验工作流监听 dev 的 core/CLI/Tauri/UI/脚本/样本/schema 变更并保留手动入口；只有引擎准备、Rust/UI 测试、依赖检查和最终 ZIP 冒烟全部通过才上传 artifact。附校验和、对应源码和许可，不创建 Release。GUI 需要已安装的 WebView2 |

## Linux

Linux 尚未进入实施范围，也没有安装包、桌面入口或运行时支持声明。未来若启动移植，会先单独确认图片引擎许可证、桌面集成和数据路径，再建立平台接口。

## 支持声明原则

代码中存在平台分支不等于平台已经支持。只有完成真实构建、安装、运行、格式处理和用户入口验收，并在版本说明中记录证据，才会把平台写入公开下载范围。

## Windows 运行时约定

ZIP 根为 Yali-workbench.exe、yali.exe、yali-runtime.json、build-info.json、README-Windows.txt 和 LICENSE。libvips 保留 runtime/vips 下的 bin / 插件 / etc / share 结构；专用工具和可再分发 CRT 位于 runtime/bin，第三方说明位于 licenses。包可移动，路径可包含空格与中文，工具不从当前目录或外部 PATH 解析；清单缺件、摘要或架构不符时失败。

数据优先使用绝对路径 YALI_HOME，默认 USERPROFILE/.yali，HOME 仅兼容回退。旧 HOME 数据不自动移动或删除；需要沿用时显式设置 YALI_HOME。下载目录由系统 Known Folder 解析。CLI、GUI 的编码、预览和尺寸探测共用运行时环境，不修改用户 PATH。

Windows 11 x64 是本轮人工验收目标；windows-2025 Actions 是服务器环境，不能代替桌面验收。无 FFmpeg 不阻断基本处理与视觉对比，仅隐藏 SSIM。WebView2 缺失时参照 [Microsoft 官方安装页](https://developer.microsoft.com/microsoft-edge/webview2/)；不静默安装浏览器或 VC++ 系统组件。未验证 Windows 10、ARM64/x86、离线共享和任意超长路径。
