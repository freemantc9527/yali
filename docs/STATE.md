# 当前项目状态

更新日期：2026-11-23。本文只记录当前事实，不保存逐任务历史。开发过程和验收证据留在本地 `.tasks/`。

## 已实现

- Rust 内核、CLI、Tauri 壳层和 React 工作台已经连通。
- Finder 提供“鸭梨 · 压一下”和“鸭梨 · 转 JPG”两条快速操作，JPEG 不显示转 JPG 动作。
- 支持桌面批量队列、预设管理、尺寸调整、输出策略、撤销清单和原图 / 产物对比。
- 默认数据根为 `~/.yali/`，配置、预设、撤销记录和缓存统一管理，支持 `YALI_HOME` 隔离。
- App 构建脚本会生成自包含运行时、真实前端和 ad-hoc 签名包。
- 剪贴板、watch、托盘、系统通知和打开文件入口已经接入代码。

## 当前待验收

- Finder 菜单在不同系统设置和干净账户中的可见性。
- 原生工作台的选择、拖入、窗口尺寸、键盘操作、预设写盘和对比手感。
- 托盘菜单、剪贴板去重、watch 文件稳定后触发和通知外观。
- 图片产物在 Preview、浏览器、微信等常用应用中的兼容性。
- Apple Silicon 以外架构的真实交互，以及公开下载后的 macOS 安全提示。

自动化测试、浏览器演示和隔离冒烟不能把以上项目标记为人工已通过。

## 平台与发布限制

macOS 是当前主平台。当前本机验证的引擎最低部署目标为 macOS 27.2，低版本系统未验证。Windows 的基本图片处理、包内引擎解析和 ZIP 构建脚本已在本地补齐；新增 Windows 构建尚未上云执行，没有可公开下载的 Windows 资产。

当前版本为 `0.1.0-alpha`，已通过 GitHub Actions 双架构构建并[公开预发布](https://github.com/freemantc9527/yali/releases/tag/v0.1.0-alpha)。两个 DMG 已下载回读并校验 SHA-256；原生交互待验项目保持不变。

Windows ZIP 实验工作流已在 [Actions 45133346294](https://github.com/freemantc9527/yali/actions/runs/45133346294) 通过：Rust 测试程序编译、UI 78 项测试、App / CLI release 编译、ZIP 打包和解压后的 CLI 版本检查。下载回读确认两个程序均为 x86_64 PE，ZIP 完整性、提交记录和 SHA-256 匹配。图片引擎没有内嵌，Rust 测试未执行，GUI 启动和图片处理仍需真实 Windows 验证，不作为正式支持声明。

Windows 运行时当前本地验证：Rust 108 项、UI 79 项、Python 打包/发布检查 23 项通过；macOS 完整 App、隔离冒烟及 default / jpg 金样通过。新增工作流会源码构建专用引擎、检查 x64 PE 依赖并对最终解压 ZIP 做六类输入和质量测试，同时提供对应源码与许可。上述 Windows 自动化尚未在 runner 执行，不能据此声明 Windows 图片处理已验收；Windows 11 GUI、WebView2 和 SmartScreen 仍待实机验收。

## 下一步

优先完成现有 macOS 入口的人工验收，补齐真实产物兼容性记录，再决定失败件队列、统计页和 Windows 构建的版本范围。新工作统一通过 `.tasks/` 进入，完成后更新 CHANGELOG 和本文件。
