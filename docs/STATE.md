# 当前项目状态

更新日期：2026-11-24。本文只记录当前事实，不保存逐任务历史。开发过程和验收证据留在本地 `.tasks/`。

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

macOS 是当前主平台。本机验证的引擎最低部署目标为 macOS 27.2；公开云端构建的两份 DMG 最低系统为 macOS 15.0，低版本真实交互未验证。Windows x64 含图片引擎的实验 ZIP 已通过云端构建与自动处理验收，现与两份 macOS DMG 在同一 Release 分别下载。

当前版本 [`0.2.0-alpha`](https://github.com/freemantc9527/yali/releases/tag/v0.2.0-alpha) 已公开预发布，提交为 `cd535bb5b998f3a6e7de5050ef3c93c4cbcac987`。[标签发布运行 45354515710](https://github.com/freemantc9527/yali/actions/runs/45354515710) 的三架构构建、隔离图片处理、质量检查、联合汇总和公开资产回读全部通过。本地再次下载全部 11 份公开资产，10 条 SHA-256、两份 DMG 与 Windows ZIP/对应源码/报告回读通过。Intel 首次上传因 GitHub artifact 服务超时失败，同运行重跑后成功，未移动标签。三个架构的程序分别下载，Windows 程序约 27.6 MiB，对应源码单独提供；旧版 `0.1.0-alpha` 保留。原生交互待验项目保持不变。

Windows 在 [Actions 45321363937](https://github.com/freemantc9527/yali/actions/runs/45321363937) 通过：实际执行 Rust 与 79 项 UI 测试、App / CLI release 编译、专用图片引擎构建、79 个 PE 文件及 DLL 依赖检查。最终 ZIP 在中文空格路径、无 HOME、受限 PATH 下完成六类输入处理、透明图片白底转换、PNG 无损、动画/多页保护、配置保存、撤销、缺 EXE/DLL 负例和独立 FFmpeg 质量对照。下载回读确认二进制/对应源码 ZIP、锁文件、来源和 SHA-256 匹配。

本地回归：Rust 109 项、UI 79 项、Python 打包/发布检查 31 项通过；macOS 完整 App、隔离冒烟及 default / jpg 金样通过。[联合云端演练 45339644244](https://github.com/freemantc9527/yali/actions/runs/45339644244) 已从同一提交 `b261b47` 成功构建 macOS arm64/x86_64 DMG 与 Windows x64 实验 ZIP，并完成同运行来源、对应源码、报告及校验和汇总。下载回读 `desktop-release-verified` 后，10 条 SHA-256、两份 DMG 和 Windows ZIP 与对应源码均通过校验；手动演练的公开发布任务保持跳过。Windows 11 GUI、WebView2 缺失和 SmartScreen 仍待实机验收，服务器自动化不能替代桌面验收。

## 下一步

优先完成现有 macOS 入口的人工验收，补齐真实产物兼容性记录，再决定失败件队列、统计页和 Windows 构建的版本范围。新工作统一通过 `.tasks/` 进入，完成后更新 CHANGELOG 和本文件。
